//! # eh3d-parser-gerber
//!
//! A tolerant reader for Gerber files (RS-274X and the X2 attributes).
//!
//! [`parse`] never panics and never stops at the first problem: what it cannot understand becomes a
//! [`Diagnostic`] with its line number and the reading goes on. The result is a flat list of
//! [`Command`]s (apertures selected, moves, flashes, lines, arcs, regions, polarity) with the
//! coordinates already converted to numbers in the units of the file.
//!
//! What the reader gives besides the list of commands: block apertures (`AB`) as apertures that hold
//! their commands, the layer transforms `LM`/`LR`/`LS` as operations, the image parameters (`IP`, `OF`, `SF`,
//! `MI`, `IR`, `AS`), incremental coordinates, the attributes of X2 and X3 (also those written as `G04 #@!`
//! comments), and the dialect of the program that wrote the file. [`macros`] evaluates the aperture macros,
//! [`image`] puts the blocks, the step and repeat and the transforms in place and [`attributes`], [`dialect`],
//! [`arc`] and [`snapshot`] are the typed attributes, the recognition of the CAD programs, the geometry of
//! an arc and the normalised text that the tests of the corpus compare.
//!
//! What is **not** done: no geometry is calculated (no arc flattening, no polygon union): that belongs to
//! the geometry crate and the CAM engine.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::string_slice)]

use std::collections::BTreeMap;

pub mod arc;
pub mod attributes;
pub mod dialect;
pub mod image;
pub mod macros;
pub mod snapshot;

pub use dialect::Dialect;

pub use eh3d_parser_common::{codes, Diagnostic, Severity, Units};
use eh3d_parser_common::{
    coordinate_error_code, decode_input, parse_f64, resolve_units, ZeroSuppression,
};
pub use eh3d_parser_common::{read_file_limited, InputError, InputLimits, ReadError};

/// Which zeros a coordinate omits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroOmission {
    /// Leading zeros are omitted (the only form current Gerber allows).
    Leading,
    /// Trailing zeros are omitted (deprecated).
    Trailing,
}

/// The coordinate format of the file (`FS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoordinateFormat {
    /// Which zeros are omitted.
    pub zero_omission: ZeroOmission,
    /// Digits before the decimal point.
    pub integer_digits: u32,
    /// Digits after the decimal point.
    pub decimal_digits: u32,
}

/// A point in the units of the file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X.
    pub x: f64,
    /// Y.
    pub y: f64,
}

/// An aperture definition (`AD`).
#[derive(Debug, Clone, PartialEq)]
pub enum Aperture {
    /// A circle.
    Circle {
        /// Diameter.
        diameter: f64,
        /// Diameter of the hole, if it has one.
        hole: Option<f64>,
    },
    /// A rectangle.
    Rectangle {
        /// Size along X.
        x: f64,
        /// Size along Y.
        y: f64,
        /// Diameter of the hole, if it has one.
        hole: Option<f64>,
    },
    /// An obround (a rectangle with round ends).
    Obround {
        /// Size along X.
        x: f64,
        /// Size along Y.
        y: f64,
        /// Diameter of the hole, if it has one.
        hole: Option<f64>,
    },
    /// A regular polygon.
    Polygon {
        /// Diameter of the circle that touches the vertices.
        diameter: f64,
        /// Number of vertices (3 to 12).
        vertices: u32,
        /// Rotation in degrees.
        rotation: f64,
        /// Diameter of the hole, if it has one.
        hole: Option<f64>,
    },
    /// An aperture defined by a macro (`AM`), with the parameters given in the definition.
    Macro {
        /// The name of the macro.
        name: String,
        /// The parameters.
        params: Vec<f64>,
    },
    /// A block aperture (`AB`): the commands between `%ABDnn*%` and `%AB*%`, drawn at each flash.
    Block {
        /// The commands of the block, in its own coordinates.
        commands: Vec<Command>,
    },
}

/// An aperture macro (`AM`): the text of its primitives, not interpreted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroDefinition {
    /// The name.
    pub name: String,
    /// The lines of the body, without the trailing `*`.
    pub body: Vec<String>,
}

/// The polarity of what follows (`LP`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Draw (add copper).
    Dark,
    /// Clear (remove what was drawn).
    Clear,
}

/// The quadrant mode of arcs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuadrantMode {
    /// Arcs never span more than 90 degrees (`G74`, deprecated).
    Single,
    /// Arcs may span up to 360 degrees (`G75`).
    Multi,
}

/// The mirroring of the apertures that are flashed (`LM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirror {
    /// No mirroring (`N`).
    None,
    /// Mirrored about the Y axis, so X changes sign (`X`).
    X,
    /// Mirrored about the X axis, so Y changes sign (`Y`).
    Y,
    /// Both (`XY`).
    Xy,
}

/// The parameters that apply to the whole image: `IP`, `OF`, `SF`, `MI`, `IR`, `AS` and `IN`.
///
/// They are deprecated but old files still carry them. They are applied in this order by
/// [`image::resolve`]: exchange of the axes, mirroring, scale, rotation and offset.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageParams {
    /// True for a negative image (`IPNEG`): the drawing is the clear part of the layer.
    pub negative: bool,
    /// Offset of the image (`OF`).
    pub offset: (f64, f64),
    /// Scale factors of the image (`SF`).
    pub scale: (f64, f64),
    /// Mirror of the A axis (`MIA1`), which changes the sign of X.
    pub mirror_a: bool,
    /// Mirror of the B axis (`MIB1`), which changes the sign of Y.
    pub mirror_b: bool,
    /// Rotation of the image in degrees: 0, 90, 180 or 270 (`IR`).
    pub rotation: u32,
    /// True when the axes are exchanged (`ASAYBX`).
    pub swap_axes: bool,
    /// The name of the image (`IN`).
    pub name: Option<String>,
}

impl Default for ImageParams {
    fn default() -> Self {
        ImageParams {
            negative: false,
            offset: (0.0, 0.0),
            scale: (1.0, 1.0),
            mirror_a: false,
            mirror_b: false,
            rotation: 0,
            swap_axes: false,
            name: None,
        }
    }
}

impl ImageParams {
    /// True when the parameters leave the geometry as it is (the polarity and the name do not move anything).
    pub fn is_identity(&self) -> bool {
        self.offset == (0.0, 0.0)
            && self.scale == (1.0, 1.0)
            && !self.mirror_a
            && !self.mirror_b
            && self.rotation == 0
            && !self.swap_axes
    }
}

/// A step-and-repeat block (`SR`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepRepeat {
    /// Repeats along X.
    pub x_repeats: u32,
    /// Repeats along Y.
    pub y_repeats: u32,
    /// Distance between repeats along X.
    pub x_step: f64,
    /// Distance between repeats along Y.
    pub y_step: f64,
}

/// What the file asks for, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    /// Make aperture `n` the current one (`Dnn`, nn >= 10).
    SelectAperture(u32),
    /// Move without drawing (`D02`).
    Move {
        /// Where to.
        to: Point,
    },
    /// Flash the current aperture (`D03`).
    Flash {
        /// Where.
        at: Point,
    },
    /// Draw a straight line with the current aperture (`D01` in linear mode).
    Line {
        /// Start.
        from: Point,
        /// End.
        to: Point,
    },
    /// Draw an arc (`D01` in circular mode). `center_offset` is `(I, J)` as the file gives it.
    Arc {
        /// Start.
        from: Point,
        /// End.
        to: Point,
        /// The `I` and `J` offsets.
        center_offset: (f64, f64),
        /// True for clockwise (`G02`).
        clockwise: bool,
        /// The quadrant mode in force.
        quadrant: QuadrantMode,
    },
    /// Start a region (`G36`): the contour that follows is filled.
    RegionStart,
    /// End a region (`G37`).
    RegionEnd,
    /// Change the polarity.
    Polarity(Polarity),
    /// Start (`Some`) or end (`None`) a step-and-repeat block.
    StepRepeat(Option<StepRepeat>),
    /// Mirror the apertures that are flashed from here on (`LM`).
    LoadMirror(Mirror),
    /// Rotate the apertures that are flashed from here on, counterclockwise and in degrees (`LR`).
    LoadRotation(f64),
    /// Scale the apertures that are flashed from here on (`LS`).
    LoadScale(f64),
}

/// An [`Operation`] and the line it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Command {
    /// The line in the file.
    pub line: usize,
    /// The operation.
    pub op: Operation,
}

/// A file attribute (`TF`, `TA`, `TO`) or its deletion (`TD`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// `TF`, `TA`, `TO` or `TD`.
    pub kind: String,
    /// The attribute name, for example `.FileFunction`.
    pub name: String,
    /// The values.
    pub values: Vec<String>,
}

/// Everything read from a Gerber file.
#[derive(Debug, Clone, PartialEq)]
pub struct GerberFile {
    /// The units (`MO`), when the file says them.
    pub units: Option<Units>,
    /// The coordinate format (`FS`), when the file says it.
    pub format: Option<CoordinateFormat>,
    /// The apertures by their number.
    pub apertures: BTreeMap<u32, Aperture>,
    /// The aperture macros by name.
    pub macros: BTreeMap<String, MacroDefinition>,
    /// The attributes in the order they appear.
    pub attributes: Vec<Attribute>,
    /// The operations in order.
    pub commands: Vec<Command>,
    /// Whether the file ends with `M02`.
    pub ended: bool,
    /// The parameters of the whole image (`IP`, `OF`, `SF`, `MI`, `IR`, `AS`, `IN`).
    pub image: ImageParams,
    /// The program that seems to have written the file, from its header.
    pub dialect: Dialect,
    /// What the reader noticed.
    pub diagnostics: Vec<Diagnostic>,
}

/// The limits of one reading: what is past a limit is not kept, the reading stops with a
/// `limit-exceeded` error and what was read until then is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLimits {
    /// The most commands (moves, flashes, lines, arcs and the like), counting those of the blocks.
    pub max_commands: usize,
    /// The most apertures, counting the block apertures.
    pub max_apertures: usize,
    /// The most aperture macros.
    pub max_macros: usize,
    /// The most attributes.
    pub max_attributes: usize,
    /// The deepest nesting of block apertures.
    pub max_block_depth: usize,
    /// The most copies a step and repeat may make (its repeats along X times along Y).
    pub max_repeat_copies: u64,
}

impl Default for ParseLimits {
    fn default() -> Self {
        ParseLimits {
            max_commands: 20_000_000,
            max_apertures: 1_000_000,
            max_macros: 100_000,
            max_attributes: 1_000_000,
            max_block_depth: 16,
            max_repeat_copies: 1_000_000,
        }
    }
}

impl GerberFile {
    /// The diagnostics of at least `severity`.
    pub fn diagnostics_from(&self, severity: Severity) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(move |d| d.severity >= severity)
    }

    /// The units of the file, or millimetres (with the `units-missing` warning in the
    /// diagnostics) when it does not declare any.
    pub fn effective_units(&self) -> Units {
        self.units.unwrap_or(eh3d_parser_common::ASSUMED_UNITS)
    }

    /// True when the file has no diagnostic of severity `Error`.
    pub fn is_clean(&self) -> bool {
        self.diagnostics_from(Severity::Error).next().is_none()
    }
}

struct State {
    file: GerberFile,
    x: Option<f64>,
    y: Option<f64>,
    mode: Mode,
    quadrant: QuadrantMode,
    in_region: bool,
    aperture: Option<u32>,
    /// The last D01, D02 or D03: coordinates without a D code of their own (old files) repeat it.
    last_d: Option<u32>,
    /// The end point of the last move, flash, line or arc.
    last_point: Option<Point>,
    /// Whether the diagnostics were cut at [`MAX_DIAGNOSTICS`].
    diagnostics_cut: bool,
    limits: ParseLimits,
    /// True after a limit was reached: the reading stops.
    stopped: bool,
    total_commands: usize,
    /// Coordinates are increments of the last point (`G91`, `FS... I`).
    incremental: bool,
    /// The block apertures that are open, the innermost last.
    blocks: Vec<BlockBuild>,
    /// Whether the text after `M02` was told.
    after_end_told: bool,
}

/// A block aperture while it is read.
struct BlockBuild {
    number: u32,
    commands: Vec<Command>,
    saved_aperture: Option<u32>,
    saved_x: Option<f64>,
    saved_y: Option<f64>,
    saved_last_d: Option<u32>,
    saved_last_point: Option<Point>,
    saved_in_region: bool,
}

/// The most diagnostics kept for one file: a damaged file of millions of bad blocks does not
/// fill the memory with messages.
pub const MAX_DIAGNOSTICS: usize = 100_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Linear,
    Clockwise,
    CounterClockwise,
}

impl State {
    fn diag(
        &mut self,
        line: usize,
        severity: Severity,
        code: &'static str,
        message: impl Into<String>,
    ) {
        if self.file.diagnostics.len() >= MAX_DIAGNOSTICS {
            if !self.diagnostics_cut {
                self.diagnostics_cut = true;
                self.file.diagnostics.push(Diagnostic::new(
                    line,
                    Severity::Error,
                    codes::TOO_MANY_DIAGNOSTICS,
                    format!("more than {MAX_DIAGNOSTICS} diagnostics: the rest are not kept"),
                ));
            }
            return;
        }
        self.file
            .diagnostics
            .push(Diagnostic::new(line, severity, code, message));
    }

    fn limit_hit(&mut self, line: usize, what: &str) {
        if !self.stopped {
            self.stopped = true;
            self.diag(
                line,
                Severity::Error,
                codes::LIMIT_EXCEEDED,
                format!("{what}: the reading stops here"),
            );
        }
    }

    fn declare_units(&mut self, units: Units, line: usize) {
        if self.file.units.is_some_and(|before| before != units) {
            self.diag(
                line,
                Severity::Error,
                codes::UNITS_CONFLICT,
                "the file declares other units than before: the last ones are used for the whole file",
            );
        }
        self.file.units = Some(units);
    }

    /// Tells, once, that something that is not a comment follows `M02`; it is still read.
    fn note_after_end(&mut self, line: usize) {
        if self.file.ended && !self.after_end_told {
            self.after_end_told = true;
            self.diag(
                line,
                Severity::Warning,
                codes::TEXT_AFTER_END,
                "there are commands after M02: they were read too (two files in one, or a damaged file)",
            );
        }
    }

    fn push(&mut self, line: usize, op: Operation) {
        if self.total_commands >= self.limits.max_commands {
            self.limit_hit(line, "more commands than the limit");
            return;
        }
        self.total_commands += 1;
        match &op {
            Operation::Move { to } => self.last_point = Some(*to),
            Operation::Flash { at } => self.last_point = Some(*at),
            Operation::Line { to, .. } | Operation::Arc { to, .. } => self.last_point = Some(*to),
            _ => {}
        }
        let command = Command { line, op };
        match self.blocks.last_mut() {
            Some(block) => block.commands.push(command),
            None => self.file.commands.push(command),
        }
    }
}

/// Reads a file from bytes: UTF-8 (with or without a byte order mark) or, with a warning
/// (`input-not-utf8`), Latin-1. An input above the default limit of 256 MiB is refused.
pub fn parse_bytes(bytes: &[u8]) -> Result<GerberFile, InputError> {
    parse_bytes_with(bytes, &InputLimits::default())
}

/// Like [`parse_bytes`] with a limit of your own.
pub fn parse_bytes_with(bytes: &[u8], limits: &InputLimits) -> Result<GerberFile, InputError> {
    parse_bytes_with_limits(bytes, limits, &ParseLimits::default())
}

/// Like [`parse_bytes`] with a limit on the size of the input and the limits of the reading of your own.
pub fn parse_bytes_with_limits(
    bytes: &[u8],
    limits: &InputLimits,
    parse_limits: &ParseLimits,
) -> Result<GerberFile, InputError> {
    let input = decode_input(bytes, limits)?;
    let mut file = parse_with_limits(&input.text, parse_limits);
    file.diagnostics.splice(0..0, input.diagnostics);
    Ok(file)
}

/// Reads a Gerber file. Never fails: problems are in [`GerberFile::diagnostics`].
pub fn parse(text: &str) -> GerberFile {
    parse_with_limits(text, &ParseLimits::default())
}

/// Reads a Gerber file with the limits given. Never fails and never panics: a limit that is reached stops
/// the reading with a `limit-exceeded` error and what was read until then is returned.
pub fn parse_with_limits(text: &str, limits: &ParseLimits) -> GerberFile {
    let mut st = State {
        file: GerberFile {
            units: None,
            format: None,
            apertures: BTreeMap::new(),
            macros: BTreeMap::new(),
            attributes: Vec::new(),
            commands: Vec::new(),
            ended: false,
            image: ImageParams::default(),
            dialect: dialect::detect(text),
            diagnostics: Vec::new(),
        },
        limits: limits.clone(),
        stopped: false,
        total_commands: 0,
        incremental: false,
        blocks: Vec::new(),
        after_end_told: false,
        x: None,
        y: None,
        mode: Mode::Linear,
        quadrant: QuadrantMode::Multi,
        in_region: false,
        last_d: None,
        last_point: None,
        diagnostics_cut: false,
        aperture: None,
    };
    // Read by bytes: every character that decides anything (`%`, `*`, a line end) is ASCII, so
    // the positions below are always on a character boundary and the text is not copied.
    let bytes = text.as_bytes();
    let mut pos = 0;
    let mut line = 1usize;
    while pos < bytes.len() && !st.stopped {
        let c = bytes[pos];
        if c == b'\n' {
            line += 1;
            pos += 1;
        } else if c.is_ascii_whitespace() || c == 0x0B {
            pos += 1;
        } else if c >= 0x80
            && text
                .get(pos..)
                .and_then(|t| t.chars().next())
                .is_some_and(|ch| ch.is_whitespace() || ch == '\u{feff}')
        {
            pos += text
                .get(pos..)
                .and_then(|t| t.chars().next())
                .map_or(1, char::len_utf8);
        } else if c == b'%' {
            let start_line = line;
            let mut end = pos + 1;
            while end < bytes.len() && bytes[end] != b'%' {
                if bytes[end] == b'\n' {
                    line += 1;
                }
                end += 1;
            }
            let body = text.get(pos + 1..end.min(bytes.len())).unwrap_or("");
            if end >= bytes.len() {
                st.diag(
                    start_line,
                    Severity::Error,
                    codes::EXTENDED_UNCLOSED,
                    "an extended command starts with % and is never closed",
                );
            }
            st.note_after_end(start_line);
            extended(&mut st, body, start_line);
            pos = end + 1;
        } else {
            let start_line = line;
            let mut end = pos;
            while end < bytes.len() && bytes[end] != b'*' && bytes[end] != b'%' {
                if bytes[end] == b'\n' {
                    line += 1;
                }
                end += 1;
            }
            let block: String = text
                .get(pos..end)
                .unwrap_or("")
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if end >= bytes.len() || bytes[end] == b'%' {
                st.diag(
                    start_line,
                    Severity::Warning,
                    codes::BLOCK_UNCLOSED,
                    "a block is not closed by *",
                );
            }
            if !block.is_empty() {
                if !block.starts_with("G04") && block != "M02" {
                    st.note_after_end(start_line);
                }
                function_block(&mut st, &block, start_line);
            }
            pos = if end < bytes.len() && bytes[end] == b'*' {
                end + 1
            } else {
                end
            };
        }
    }
    if !st.file.ended {
        let last = line;
        st.diag(
            last,
            Severity::Warning,
            codes::END_MISSING,
            "the file does not end with M02",
        );
    }
    if st.in_region {
        let last = line;
        st.diag(
            last,
            Severity::Error,
            codes::REGION_UNCLOSED,
            "a region (G36) is never closed by G37",
        );
    }
    while let Some(block) = st.blocks.pop() {
        st.diag(
            line,
            Severity::Error,
            codes::BLOCK_APERTURE_INVALID,
            format!("the block aperture D{} is never closed by AB", block.number),
        );
    }
    let last = line;
    resolve_units(st.file.units, last, &mut st.file.diagnostics);
    st.file
}

fn extended(st: &mut State, body: &str, line: usize) {
    let blocks: Vec<String> = body
        .split('*')
        .map(|b| b.chars().filter(|c| !c.is_whitespace()).collect::<String>())
        .filter(|b| !b.is_empty())
        .collect();
    let Some(first) = blocks.first() else { return };
    let cmd = first.as_str();
    if let Some(name) = cmd.strip_prefix("AM") {
        let body: Vec<String> = blocks[1..].to_vec();
        if name.is_empty() {
            st.diag(
                line,
                Severity::Error,
                codes::MACRO_NAME_MISSING,
                "an aperture macro (AM) has no name",
            );
        } else if st.file.macros.len() >= st.limits.max_macros && !st.file.macros.contains_key(name)
        {
            st.limit_hit(line, "more aperture macros than the limit");
        } else {
            st.file.macros.insert(
                name.to_owned(),
                MacroDefinition {
                    name: name.to_owned(),
                    body,
                },
            );
        }
        return;
    }
    for block in &blocks {
        extended_one(st, block, line);
    }
}

fn extended_one(st: &mut State, cmd: &str, line: usize) {
    if let Some(rest) = cmd.strip_prefix("FS") {
        format_spec(st, rest, line);
    } else if let Some(rest) = cmd.strip_prefix("MO") {
        match rest {
            "MM" => st.declare_units(Units::Millimetres, line),
            "IN" => st.declare_units(Units::Inches, line),
            _ => st.diag(
                line,
                Severity::Error,
                codes::UNITS_UNKNOWN,
                format!("unknown units '{rest}' in MO"),
            ),
        }
    } else if let Some(rest) = cmd.strip_prefix("AD") {
        aperture_definition(st, rest, line);
    } else if let Some(rest) = cmd.strip_prefix("LP") {
        match rest {
            "D" => st.push(line, Operation::Polarity(Polarity::Dark)),
            "C" => st.push(line, Operation::Polarity(Polarity::Clear)),
            _ => st.diag(
                line,
                Severity::Error,
                codes::POLARITY_UNKNOWN,
                format!("unknown polarity '{rest}' in LP"),
            ),
        }
    } else if let Some(rest) = cmd.strip_prefix("SR") {
        step_repeat(st, rest, line);
    } else if let Some(rest) = ["TF", "TA", "TO", "TD"]
        .iter()
        .find_map(|k| cmd.strip_prefix(k).map(|r| (*k, r)))
    {
        let (kind, rest) = rest;
        attribute(st, kind, rest, line);
    } else if let Some(rest) = cmd.strip_prefix("AB") {
        block_aperture(st, rest, line);
    } else if let Some(rest) = cmd.strip_prefix("LM") {
        let mirror = match rest {
            "N" => Some(Mirror::None),
            "X" => Some(Mirror::X),
            "Y" => Some(Mirror::Y),
            "XY" => Some(Mirror::Xy),
            _ => None,
        };
        match mirror {
            Some(m) => st.push(line, Operation::LoadMirror(m)),
            None => layer_transform_invalid(st, line, cmd),
        }
    } else if let Some(rest) = cmd.strip_prefix("LR") {
        match parse_f64(rest) {
            Some(v) => st.push(line, Operation::LoadRotation(v)),
            None => layer_transform_invalid(st, line, cmd),
        }
    } else if let Some(rest) = cmd.strip_prefix("LS") {
        match parse_f64(rest) {
            Some(v) if v > 0.0 => st.push(line, Operation::LoadScale(v)),
            _ => layer_transform_invalid(st, line, cmd),
        }
    } else if ["IP", "OF", "SF", "AS", "IN", "LN", "MI", "IR", "KO", "PF"]
        .iter()
        .any(|k| cmd.starts_with(k))
    {
        image_parameter(st, cmd, line);
    } else {
        st.diag(
            line,
            Severity::Warning,
            codes::UNKNOWN_COMMAND,
            format!(
                "unknown extended command '{}'",
                cmd.chars().take(12).collect::<String>()
            ),
        );
    }
}

fn layer_transform_invalid(st: &mut State, line: usize, cmd: &str) {
    st.diag(
        line,
        Severity::Error,
        codes::LAYER_TRANSFORM_INVALID,
        format!(
            "{}: the value is not valid",
            cmd.chars().take(12).collect::<String>()
        ),
    );
}

fn attribute(st: &mut State, kind: &str, rest: &str, line: usize) {
    if st.file.attributes.len() >= st.limits.max_attributes {
        st.limit_hit(line, "more attributes than the limit");
        return;
    }
    let mut parts = rest.split(',');
    let name = parts.next().unwrap_or("").to_owned();
    if kind != "TD" && name.is_empty() {
        st.diag(
            line,
            Severity::Warning,
            codes::ATTRIBUTE_MALFORMED,
            format!("{kind} has no attribute name"),
        );
    }
    st.file.attributes.push(Attribute {
        kind: kind.to_owned(),
        name,
        values: parts.map(str::to_owned).collect(),
    });
}

/// `A<number>B<number>` in either order, each part optional.
fn ab_pair(text: &str) -> Option<(Option<f64>, Option<f64>)> {
    let (mut a, mut b) = (None, None);
    let mut current = ' ';
    let mut buffer = String::new();
    for c in text.chars().chain(std::iter::once('\0')) {
        if matches!(c, 'A' | 'B' | '\0') {
            match current {
                'A' => a = Some(parse_f64(&buffer)?),
                'B' => b = Some(parse_f64(&buffer)?),
                _ if !buffer.is_empty() => return None,
                _ => {}
            }
            buffer.clear();
            current = c;
        } else {
            buffer.push(c);
        }
    }
    Some((a, b))
}

fn image_parameter(st: &mut State, cmd: &str, line: usize) {
    let code = cmd.get(..2).unwrap_or(cmd);
    let rest = cmd.get(2..).unwrap_or("");
    let mut invalid = false;
    match code {
        "IP" => match rest {
            "POS" => st.file.image.negative = false,
            "NEG" => st.file.image.negative = true,
            _ => invalid = true,
        },
        "OF" => match ab_pair(rest) {
            Some((a, b)) => {
                st.file.image.offset = (a.unwrap_or(0.0), b.unwrap_or(0.0));
            }
            None => invalid = true,
        },
        "SF" => match ab_pair(rest) {
            Some((a, b)) if a.unwrap_or(1.0) != 0.0 && b.unwrap_or(1.0) != 0.0 => {
                st.file.image.scale = (a.unwrap_or(1.0), b.unwrap_or(1.0));
            }
            _ => invalid = true,
        },
        "MI" => match ab_pair(rest) {
            Some((a, b))
                if [a, b]
                    .iter()
                    .all(|v| matches!(v, None | Some(0.0) | Some(1.0))) =>
            {
                st.file.image.mirror_a = a == Some(1.0);
                st.file.image.mirror_b = b == Some(1.0);
            }
            _ => invalid = true,
        },
        "IR" => match rest {
            "0" | "90" | "180" | "270" => st.file.image.rotation = rest.parse().unwrap_or(0),
            _ => invalid = true,
        },
        "AS" => match rest {
            "AXBY" => st.file.image.swap_axes = false,
            "AYBX" => st.file.image.swap_axes = true,
            _ => invalid = true,
        },
        "IN" => st.file.image.name = Some(rest.to_owned()),
        // the name of a layer and the knock-out and the plot film are not geometry of the file
        _ => {}
    }
    if invalid {
        st.diag(
            line,
            Severity::Error,
            codes::IMAGE_PARAMETER_INVALID,
            format!(
                "the deprecated command {} has a value that cannot be read",
                cmd.chars().take(12).collect::<String>()
            ),
        );
    } else if matches!(code, "KO" | "PF") {
        st.diag(
            line,
            Severity::Error,
            codes::GEOMETRY_COMMAND_UNSUPPORTED,
            format!("the deprecated command {code} changes the geometry and is not supported"),
        );
    } else {
        st.diag(
            line,
            Severity::Info,
            codes::DEPRECATED_COMMAND,
            format!("the deprecated command {code} was read"),
        );
    }
}

fn block_aperture(st: &mut State, rest: &str, line: usize) {
    if rest.is_empty() {
        let Some(block) = st.blocks.pop() else {
            st.diag(
                line,
                Severity::Error,
                codes::BLOCK_APERTURE_INVALID,
                "AB closes a block aperture that was not opened",
            );
            return;
        };
        if st.in_region {
            st.diag(
                line,
                Severity::Error,
                codes::REGION_UNCLOSED,
                "a region (G36) is never closed before the end of the block aperture",
            );
        }
        st.aperture = block.saved_aperture;
        st.x = block.saved_x;
        st.y = block.saved_y;
        st.last_d = block.saved_last_d;
        st.last_point = block.saved_last_point;
        st.in_region = block.saved_in_region;
        if st.file.apertures.len() >= st.limits.max_apertures
            && !st.file.apertures.contains_key(&block.number)
        {
            st.limit_hit(line, "more apertures than the limit");
            return;
        }
        if st
            .file
            .apertures
            .insert(
                block.number,
                Aperture::Block {
                    commands: block.commands,
                },
            )
            .is_some()
        {
            st.diag(
                line,
                Severity::Warning,
                codes::APERTURE_REDEFINED,
                format!(
                    "AB: D{} is defined twice; the last definition wins",
                    block.number
                ),
            );
        }
        return;
    }
    let number = rest
        .strip_prefix('D')
        .and_then(|d| d.parse::<u32>().ok())
        .filter(|n| *n >= 10);
    let Some(number) = number else {
        st.diag(
            line,
            Severity::Error,
            codes::BLOCK_APERTURE_INVALID,
            "AB: expected D followed by a number of 10 or more",
        );
        return;
    };
    if st.blocks.len() >= st.limits.max_block_depth {
        st.limit_hit(line, "block apertures nested deeper than the limit");
        return;
    }
    st.blocks.push(BlockBuild {
        number,
        commands: Vec::new(),
        saved_aperture: st.aperture.take(),
        saved_x: st.x.take(),
        saved_y: st.y.take(),
        saved_last_d: st.last_d.take(),
        saved_last_point: st.last_point.take(),
        saved_in_region: std::mem::replace(&mut st.in_region, false),
    });
}

fn format_spec(st: &mut State, rest: &str, line: usize) {
    let bytes: Vec<char> = rest.chars().collect();
    let zero = match bytes.first() {
        Some('L') => ZeroOmission::Leading,
        Some('T') => ZeroOmission::Trailing,
        _ => {
            st.diag(
                line,
                Severity::Error,
                codes::FORMAT_ZERO_OMISSION_INVALID,
                "FS: the zero omission must be L or T",
            );
            return;
        }
    };
    if zero == ZeroOmission::Trailing {
        st.diag(
            line,
            Severity::Warning,
            codes::FORMAT_TRAILING_DEPRECATED,
            "FS: omitting trailing zeros is deprecated",
        );
    }
    match bytes.get(1) {
        Some('A') => {}
        Some('I') => {
            st.incremental = true;
            st.diag(
                line,
                Severity::Warning,
                codes::DEPRECATED_COMMAND,
                "FS: incremental coordinates are deprecated; each X and Y is added to the last point",
            );
        }
        _ => st.diag(
            line,
            Severity::Error,
            codes::FORMAT_MODE_INVALID,
            "FS: the coordinate mode must be A (absolute) or I (incremental)",
        ),
    }
    let tail: String = bytes.iter().skip(2).collect();
    let (xs, ys) = match tail.split_once('Y') {
        Some((x, y)) if x.starts_with('X') => (x.strip_prefix('X').unwrap_or(x), y),
        _ => {
            st.diag(
                line,
                Severity::Error,
                codes::FORMAT_MALFORMED,
                "FS: expected X<digits>Y<digits>",
            );
            return;
        }
    };
    let digits = |s: &str| -> Option<(u32, u32)> {
        if s.len() != 2 || !s.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut digits = s.chars().map(|c| c.to_digit(10));
        let (i, d) = (digits.next()??, digits.next()??);
        (1..=6).contains(&i).then_some(())?;
        (d <= 6).then_some((i, d))
    };
    match (digits(xs), digits(ys)) {
        (Some(x), Some(y)) => {
            if x != y {
                st.diag(
                    line,
                    Severity::Warning,
                    codes::FORMAT_AXES_DIFFER,
                    "FS: X and Y use different formats; the X format is used for both",
                );
            }
            let format = CoordinateFormat {
                zero_omission: zero,
                integer_digits: x.0,
                decimal_digits: x.1,
            };
            if st.file.format.is_some_and(|before| before != format) {
                st.diag(
                    line,
                    Severity::Error,
                    codes::FORMAT_REDEFINED,
                    "FS: the format is declared again and differently; the new one is used from here on",
                );
            }
            st.file.format = Some(format);
        }
        _ => st.diag(
            line,
            Severity::Error,
            codes::FORMAT_DIGITS_INVALID,
            "FS: the format must be two digits (1 to 6 integer, 0 to 6 decimal) for X and Y",
        ),
    }
}

fn aperture_definition(st: &mut State, rest: &str, line: usize) {
    let Some(rest) = rest.strip_prefix('D') else {
        st.diag(
            line,
            Severity::Error,
            codes::APERTURE_NUMBER_INVALID,
            "AD: the aperture number must start with D",
        );
        return;
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let Ok(number) = digits.parse::<u32>() else {
        st.diag(
            line,
            Severity::Error,
            codes::APERTURE_NUMBER_INVALID,
            "AD: missing aperture number",
        );
        return;
    };
    if number < 10 {
        st.diag(
            line,
            Severity::Error,
            codes::APERTURE_NUMBER_RESERVED,
            format!("AD: D{number} is reserved; apertures start at D10"),
        );
        return;
    }
    let spec = rest.get(digits.len()..).unwrap_or("");
    let (template, params_text) = spec.split_once(',').unwrap_or((spec, ""));
    let mut params = Vec::new();
    let mut comma_seen = false;
    for p in params_text.split('X').filter(|p| !p.is_empty()) {
        // a computer set to a language that writes the decimal point as a comma wrote some files that way
        let value = parse_f64(p).or_else(|| {
            let v = if p.contains(',') && !p.contains('.') {
                parse_f64(&p.replace(',', "."))
            } else {
                None
            };
            comma_seen |= v.is_some();
            v
        });
        match value {
            Some(v) => params.push(v),
            None => {
                st.diag(
                    line,
                    Severity::Error,
                    codes::APERTURE_PARAMETER_INVALID,
                    format!("AD: '{p}' is not a number"),
                );
                return;
            }
        }
    }
    if comma_seen {
        st.diag(
            line,
            Severity::Warning,
            codes::DECIMAL_COMMA,
            "AD: a number uses a comma as the decimal point; it was read as a point",
        );
    }
    let aperture = match template {
        "C" if !params.is_empty() => Aperture::Circle {
            diameter: params[0],
            hole: params.get(1).copied(),
        },
        "R" if params.len() >= 2 => Aperture::Rectangle {
            x: params[0],
            y: params[1],
            hole: params.get(2).copied(),
        },
        "O" if params.len() >= 2 => Aperture::Obround {
            x: params[0],
            y: params[1],
            hole: params.get(2).copied(),
        },
        "P" if params.len() >= 2 => {
            let vertices = params[1];
            if vertices.fract() != 0.0 || !(3.0..=12.0).contains(&vertices) {
                st.diag(
                    line,
                    Severity::Error,
                    codes::APERTURE_POLYGON_VERTICES,
                    "AD: a polygon has 3 to 12 vertices",
                );
                return;
            }
            Aperture::Polygon {
                diameter: params[0],
                vertices: vertices as u32,
                rotation: params.get(2).copied().unwrap_or(0.0),
                hole: params.get(3).copied(),
            }
        }
        "C" | "R" | "O" | "P" => {
            st.diag(
                line,
                Severity::Error,
                codes::APERTURE_PARAMETERS_MISSING,
                format!("AD: the {template} aperture has too few parameters"),
            );
            return;
        }
        name if !name.is_empty() => {
            if !st.file.macros.contains_key(name) {
                st.diag(
                    line,
                    Severity::Warning,
                    codes::APERTURE_MACRO_UNDEFINED,
                    format!("AD: the macro '{name}' is not defined before its use"),
                );
            }
            Aperture::Macro {
                name: name.to_owned(),
                params,
            }
        }
        _ => {
            st.diag(
                line,
                Severity::Error,
                codes::APERTURE_TEMPLATE_MISSING,
                "AD: missing aperture template",
            );
            return;
        }
    };
    if aperture_has_bad_size(&aperture) {
        st.diag(
            line,
            Severity::Warning,
            codes::APERTURE_SIZE_NEGATIVE,
            format!("AD: D{number} has a negative size"),
        );
    }
    if st.file.apertures.len() >= st.limits.max_apertures
        && !st.file.apertures.contains_key(&number)
    {
        st.limit_hit(line, "more apertures than the limit");
        return;
    }
    if st.file.apertures.insert(number, aperture).is_some() {
        st.diag(
            line,
            Severity::Warning,
            codes::APERTURE_REDEFINED,
            format!("AD: D{number} is defined twice; the last definition wins"),
        );
    }
}

fn aperture_has_bad_size(a: &Aperture) -> bool {
    match a {
        Aperture::Circle { diameter, .. } | Aperture::Polygon { diameter, .. } => *diameter < 0.0,
        Aperture::Rectangle { x, y, .. } | Aperture::Obround { x, y, .. } => *x < 0.0 || *y < 0.0,
        Aperture::Macro { .. } | Aperture::Block { .. } => false,
    }
}

fn step_repeat(st: &mut State, rest: &str, line: usize) {
    if rest.is_empty() {
        st.push(line, Operation::StepRepeat(None));
        return;
    }
    let mut x_repeats = None;
    let mut y_repeats = None;
    let mut x_step = None;
    let mut y_step = None;
    let mut current = ' ';
    let mut buffer = String::new();
    let mut fields = Vec::new();
    for c in rest.chars().chain(std::iter::once('\0')) {
        if matches!(c, 'X' | 'Y' | 'I' | 'J' | '\0') {
            if current != ' ' {
                fields.push((current, std::mem::take(&mut buffer)));
            }
            current = c;
        } else {
            buffer.push(c);
        }
    }
    for (letter, value) in fields {
        match (letter, parse_f64(&value)) {
            ('X', Some(v)) if v >= 1.0 && v.fract() == 0.0 => x_repeats = Some(v as u32),
            ('Y', Some(v)) if v >= 1.0 && v.fract() == 0.0 => y_repeats = Some(v as u32),
            ('I', Some(v)) => x_step = Some(v),
            ('J', Some(v)) => y_step = Some(v),
            _ => {
                st.diag(
                    line,
                    Severity::Error,
                    codes::STEP_REPEAT_INVALID,
                    format!("SR: bad value '{letter}{value}'"),
                );
                return;
            }
        }
    }
    if let (Some(x), Some(y)) = (x_repeats, y_repeats) {
        if u64::from(x) * u64::from(y) > st.limits.max_repeat_copies {
            st.diag(
                line,
                Severity::Error,
                codes::LIMIT_EXCEEDED,
                format!(
                    "SR: {x} by {y} copies are more than the limit of {}; the repeat is not applied",
                    st.limits.max_repeat_copies
                ),
            );
            return;
        }
    }
    match (x_repeats, y_repeats, x_step, y_step) {
        (Some(x_repeats), Some(y_repeats), Some(x_step), Some(y_step)) => st.push(
            line,
            Operation::StepRepeat(Some(StepRepeat {
                x_repeats,
                y_repeats,
                x_step,
                y_step,
            })),
        ),
        _ => st.diag(
            line,
            Severity::Error,
            codes::STEP_REPEAT_INVALID,
            "SR: needs X, Y, I and J",
        ),
    }
}

/// Splits a function block such as `G01X100Y-200D01` into letter and digits pairs.
fn words(block: &str) -> Vec<(char, String)> {
    let mut out: Vec<(char, String)> = Vec::new();
    for c in block.chars() {
        if c.is_ascii_alphabetic() {
            out.push((c.to_ascii_uppercase(), String::new()));
        } else if let Some(last) = out.last_mut() {
            last.1.push(c);
        }
    }
    out
}

fn function_block(st: &mut State, block: &str, line: usize) {
    // some programs (KiCad before X2 was accepted everywhere) write the attributes as comments: G04 #@! TF.FileFunction,...
    if let Some(rest) = block.strip_prefix("G04#@!") {
        if let Some((kind, rest)) = ["TF", "TA", "TO", "TD"]
            .iter()
            .find_map(|k| rest.strip_prefix(k).map(|r| (*k, r)))
        {
            attribute(st, kind, rest, line);
        }
        return;
    }
    if block.starts_with("G04") {
        return;
    }
    let words = words(block);
    if words.is_empty() {
        st.diag(
            line,
            Severity::Warning,
            codes::BLOCK_EMPTY,
            "a block has no letters and was ignored",
        );
        return;
    }
    if !block.starts_with(|c: char| c.is_ascii_alphabetic()) {
        st.diag(
            line,
            Severity::Error,
            codes::CHARACTER_IGNORED,
            "what stands before the first letter of a block was left out",
        );
    }
    let (mut x, mut y, mut i, mut j) = (None, None, None, None);
    let mut d: Option<u32> = None;
    for (letter, digits) in &words {
        match *letter {
            'G' => g_code(st, digits, line),
            'M' => match digits.parse::<u32>() {
                Ok(2) => st.file.ended = true,
                Ok(0) | Ok(1) => st.diag(
                    line,
                    Severity::Info,
                    codes::DEPRECATED_COMMAND,
                    "M00/M01 are deprecated and were ignored",
                ),
                _ => st.diag(
                    line,
                    Severity::Warning,
                    codes::UNKNOWN_M_CODE,
                    format!("unknown M{digits}"),
                ),
            },
            'X' | 'Y' | 'I' | 'J' => match coordinate(st, digits, line) {
                Some(v) => {
                    let slot = match *letter {
                        'X' => &mut x,
                        'Y' => &mut y,
                        'I' => &mut i,
                        _ => &mut j,
                    };
                    if slot.replace(v).is_some() {
                        word_repeated(st, *letter, line);
                    }
                }
                None => return,
            },
            'D' => match digits.parse::<u32>() {
                Ok(n) => {
                    if d.replace(n).is_some() {
                        word_repeated(st, 'D', line);
                    }
                }
                Err(_) => {
                    st.diag(
                        line,
                        Severity::Error,
                        codes::D_CODE_INVALID,
                        format!("'D{digits}' is not an operation or aperture number"),
                    );
                    return;
                }
            },
            'N' => {}
            other => {
                st.diag(
                    line,
                    Severity::Warning,
                    codes::UNKNOWN_WORD,
                    format!("unknown word '{other}{digits}' was ignored"),
                );
            }
        }
    }
    if let Some(v) = x {
        st.x = Some(if st.incremental {
            st.x.unwrap_or(0.0) + v
        } else {
            v
        });
    }
    if let Some(v) = y {
        st.y = Some(if st.incremental {
            st.y.unwrap_or(0.0) + v
        } else {
            v
        });
    }
    let d = match d {
        Some(d) => Some(d),
        None if x.is_some() || y.is_some() => {
            match st.last_d {
                Some(last) => {
                    st.diag(line, Severity::Warning, codes::IMPLIED_OPERATION, format!("coordinates without D01, D02 or D03: the last one (D{last:02}) was used (the deprecated implied operation)"));
                    Some(last)
                }
                None => {
                    st.diag(line, Severity::Error, codes::OPERATION_MISSING, "coordinates without D01, D02 or D03 and no operation before them to repeat");
                    None
                }
            }
        }
        None => None,
    };
    let Some(d) = d else {
        return;
    };
    if (1..=3).contains(&d) {
        st.last_d = Some(d);
    }
    match d {
        1..=3 => {
            let (Some(cx), Some(cy)) = (st.x, st.y) else {
                st.diag(
                    line,
                    Severity::Error,
                    codes::COORDINATE_MISSING,
                    "an operation uses a coordinate that was never given",
                );
                return;
            };
            let to = Point { x: cx, y: cy };
            match d {
                2 => {
                    st.push(line, Operation::Move { to });
                    return;
                }
                3 => {
                    if st.aperture.is_none() {
                        st.diag(
                            line,
                            Severity::Error,
                            codes::APERTURE_NOT_SELECTED,
                            "a flash (D03) happens before any aperture is selected",
                        );
                    } else if !st.file.apertures.contains_key(&st.aperture.unwrap_or(0)) {
                        st.diag(
                            line,
                            Severity::Error,
                            codes::APERTURE_UNDEFINED,
                            "a flash uses an aperture that is not defined",
                        );
                    }
                    st.push(line, Operation::Flash { at: to });
                    return;
                }
                _ => {}
            }
            let from = current_point(st, line, &words);
            if !st.in_region {
                if st.aperture.is_none() {
                    st.diag(
                        line,
                        Severity::Error,
                        codes::APERTURE_NOT_SELECTED,
                        "a draw (D01) happens before any aperture is selected",
                    );
                } else if !st.file.apertures.contains_key(&st.aperture.unwrap_or(0)) {
                    st.diag(
                        line,
                        Severity::Error,
                        codes::APERTURE_UNDEFINED,
                        "a draw uses an aperture that is not defined",
                    );
                }
            }
            let Some(from) = from else { return };
            let op = match st.mode {
                Mode::Linear => Operation::Line { from, to },
                mode => Operation::Arc {
                    from,
                    to,
                    center_offset: (i.unwrap_or(0.0), j.unwrap_or(0.0)),
                    clockwise: mode == Mode::Clockwise,
                    quadrant: st.quadrant,
                },
            };
            st.push(line, op);
        }
        n if n >= 10 => {
            st.aperture = Some(n);
            if !st.file.apertures.contains_key(&n) {
                st.diag(
                    line,
                    Severity::Warning,
                    codes::APERTURE_UNDEFINED,
                    format!("D{n} is selected but not defined (yet)"),
                );
            }
            st.push(line, Operation::SelectAperture(n));
        }
        n => st.diag(
            line,
            Severity::Error,
            codes::D_CODE_INVALID,
            format!("D{n:02} is not a valid operation or aperture"),
        ),
    }
}

fn word_repeated(st: &mut State, letter: char, line: usize) {
    st.diag(
        line,
        Severity::Error,
        codes::WORD_REPEATED,
        format!("{letter} is written twice in a block: only the last one is used"),
    );
}

/// The point a draw starts from: the point before this block's coordinates were applied.
fn current_point(st: &mut State, line: usize, _words: &[(char, String)]) -> Option<Point> {
    // The previous operation that carried a coordinate left its end point in `last_point`.
    let last = st.last_point;
    if last.is_none() {
        st.diag(
            line,
            Severity::Error,
            codes::START_POINT_MISSING,
            "a draw starts before any move gave a start point",
        );
    }
    last
}

fn g_code(st: &mut State, digits: &str, line: usize) {
    match digits.parse::<u32>() {
        Ok(1) => st.mode = Mode::Linear,
        Ok(2) => st.mode = Mode::Clockwise,
        Ok(3) => st.mode = Mode::CounterClockwise,
        Ok(36) => {
            if st.in_region {
                st.diag(
                    line,
                    Severity::Error,
                    codes::REGION_NESTED,
                    "G36 inside a region",
                );
            }
            st.in_region = true;
            st.push(line, Operation::RegionStart);
        }
        Ok(37) => {
            if !st.in_region {
                st.diag(
                    line,
                    Severity::Error,
                    codes::REGION_NOT_OPEN,
                    "G37 without G36",
                );
            }
            st.in_region = false;
            st.push(line, Operation::RegionEnd);
        }
        Ok(74) => {
            st.quadrant = QuadrantMode::Single;
            st.diag(
                line,
                Severity::Warning,
                codes::DEPRECATED_COMMAND,
                "G74 (single quadrant) is deprecated",
            );
        }
        Ok(75) => st.quadrant = QuadrantMode::Multi,
        Ok(70) => {
            st.declare_units(Units::Inches, line);
            st.diag(
                line,
                Severity::Warning,
                codes::DEPRECATED_COMMAND,
                "G70 is deprecated: use MOIN",
            );
        }
        Ok(71) => {
            st.declare_units(Units::Millimetres, line);
            st.diag(
                line,
                Severity::Warning,
                codes::DEPRECATED_COMMAND,
                "G71 is deprecated: use MOMM",
            );
        }
        Ok(54) | Ok(55) => st.diag(
            line,
            Severity::Info,
            codes::DEPRECATED_COMMAND,
            "G54/G55 are deprecated and were ignored",
        ),
        Ok(90) => st.incremental = false,
        Ok(91) => {
            st.incremental = true;
            st.diag(
                line,
                Severity::Warning,
                codes::DEPRECATED_COMMAND,
                "G91 (incremental coordinates) is deprecated; each X and Y is added to the last point",
            );
        }
        _ => st.diag(
            line,
            Severity::Warning,
            codes::UNKNOWN_G_CODE,
            format!("unknown G{digits}"),
        ),
    }
}

fn coordinate(st: &mut State, digits: &str, line: usize) -> Option<f64> {
    let Some(format) = st.file.format else {
        st.diag(
            line,
            Severity::Error,
            codes::COORDINATE_BEFORE_FORMAT,
            "a coordinate appears before the format (FS) is given",
        );
        return None;
    };
    let with_point;
    let mut digits = digits;
    if digits.contains(',') && !digits.contains('.') {
        with_point = digits.replace(',', ".");
        if decode_coordinate(&with_point, format).is_some() {
            st.diag(
                line,
                Severity::Warning,
                codes::DECIMAL_COMMA,
                format!("'{digits}' uses a comma as the decimal point; it was read as a point"),
            );
            digits = &with_point;
        }
    }
    match decode_coordinate(digits, format) {
        Some(v) => Some(v),
        None => {
            st.diag(
                line,
                Severity::Error,
                coordinate_error_code(digits, format.integer_digits, format.decimal_digits),
                format!("'{digits}' is not a valid coordinate"),
            );
            None
        }
    }
}

/// Converts the digits of a coordinate to a number following the file's format.
///
/// A value with a decimal point is read as it is. Otherwise the digits are an integer scaled by
/// the format: leading-zero omission divides by 10^decimals; trailing-zero omission first pads the
/// digits on the right to the full width.
pub fn decode_coordinate(text: &str, format: CoordinateFormat) -> Option<f64> {
    let zeros = match format.zero_omission {
        ZeroOmission::Leading => ZeroSuppression::Leading,
        ZeroOmission::Trailing => ZeroSuppression::Trailing,
    };
    eh3d_parser_common::decode_coordinate(text, format.integer_digits, format.decimal_digits, zeros)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORMAT_24: CoordinateFormat = CoordinateFormat {
        zero_omission: ZeroOmission::Leading,
        integer_digits: 2,
        decimal_digits: 4,
    };

    #[test]
    fn coordinates_follow_the_format() {
        assert_eq!(decode_coordinate("999999", FORMAT_24), Some(99.9999));
        assert_eq!(
            decode_coordinate("1000000", FORMAT_24),
            None,
            "seven digits do not fit a 2.4 format"
        );
        assert_eq!(decode_coordinate("-5000", FORMAT_24), Some(-0.5));
        assert_eq!(decode_coordinate("12.5", FORMAT_24), Some(12.5));
        assert_eq!(decode_coordinate("0", FORMAT_24), Some(0.0));
        let trailing = CoordinateFormat {
            zero_omission: ZeroOmission::Trailing,
            ..FORMAT_24
        };
        assert_eq!(decode_coordinate("12", trailing), Some(12.0));
        assert_eq!(decode_coordinate("1", trailing), Some(10.0));
        for bad in ["", "-", "1a", "--1", "1.2.3", "99999999999999999"] {
            assert_eq!(decode_coordinate(bad, FORMAT_24), None, "{bad}");
        }
    }
}
