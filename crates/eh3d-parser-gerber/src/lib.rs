//! # eh3d-parser-gerber
//!
//! A tolerant reader for Gerber files (RS-274X and the X2 attributes).
//!
//! [`parse`] never panics and never stops at the first problem: what it cannot understand becomes a
//! [`Diagnostic`] with its line number and the reading goes on. The result is a flat list of
//! [`Command`]s (apertures selected, moves, flashes, lines, arcs, regions, polarity) with the
//! coordinates already converted to numbers in the units of the file.
//!
//! What is **not** done: aperture macro bodies are kept as text and not interpreted, block
//! apertures (`AB`) and the layer transforms `LM`/`LR`/`LS` are reported as unsupported, and no
//! geometry is calculated (no arc flattening, no polygon union): that belongs to the CAM engine.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something worth knowing (a deprecated command that was ignored).
    Info,
    /// The file is probably still read correctly, but it is not clean.
    Warning,
    /// Something was lost: the result may be wrong around it.
    Error,
}

/// Something the reader noticed, with the line where it happened (counting from 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The line in the file.
    pub line: usize,
    /// How serious it is.
    pub severity: Severity,
    /// What happened, in English.
    pub message: String,
}

/// The units of the coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    /// Millimetres.
    Millimetres,
    /// Inches.
    Inches,
}

impl Units {
    /// Converts a length in these units to millimetres.
    pub fn to_mm(self, value: f64) -> f64 {
        match self {
            Units::Millimetres => value,
            Units::Inches => value * 25.4,
        }
    }
}

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
    /// What the reader noticed.
    pub diagnostics: Vec<Diagnostic>,
}

impl GerberFile {
    /// The diagnostics of at least `severity`.
    pub fn diagnostics_from(&self, severity: Severity) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(move |d| d.severity >= severity)
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
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Linear,
    Clockwise,
    CounterClockwise,
}

impl State {
    fn diag(&mut self, line: usize, severity: Severity, message: impl Into<String>) {
        self.file.diagnostics.push(Diagnostic {
            line,
            severity,
            message: message.into(),
        });
    }

    fn push(&mut self, line: usize, op: Operation) {
        self.file.commands.push(Command { line, op });
    }
}

/// Reads a Gerber file. Never fails: problems are in [`GerberFile::diagnostics`].
pub fn parse(text: &str) -> GerberFile {
    let mut st = State {
        file: GerberFile {
            units: None,
            format: None,
            apertures: BTreeMap::new(),
            macros: BTreeMap::new(),
            attributes: Vec::new(),
            commands: Vec::new(),
            ended: false,
            diagnostics: Vec::new(),
        },
        x: None,
        y: None,
        mode: Mode::Linear,
        quadrant: QuadrantMode::Multi,
        in_region: false,
        aperture: None,
    };
    let chars: Vec<char> = text.chars().collect();
    let mut pos = 0;
    let mut line = 1usize;
    while pos < chars.len() {
        let c = chars[pos];
        if c == '\n' {
            line += 1;
            pos += 1;
        } else if c.is_whitespace() || c == '\u{feff}' {
            pos += 1;
        } else if c == '%' {
            let start_line = line;
            let mut end = pos + 1;
            while end < chars.len() && chars[end] != '%' {
                if chars[end] == '\n' {
                    line += 1;
                }
                end += 1;
            }
            let body: String = chars[pos + 1..end.min(chars.len())].iter().collect();
            if end >= chars.len() {
                st.diag(
                    start_line,
                    Severity::Error,
                    "an extended command starts with % and is never closed",
                );
            }
            extended(&mut st, &body, start_line);
            pos = end + 1;
        } else {
            let start_line = line;
            let mut end = pos;
            while end < chars.len() && chars[end] != '*' && chars[end] != '%' {
                if chars[end] == '\n' {
                    line += 1;
                }
                end += 1;
            }
            let block: String = chars[pos..end]
                .iter()
                .filter(|c| !c.is_whitespace())
                .collect();
            if end >= chars.len() || chars[end] == '%' {
                st.diag(start_line, Severity::Warning, "a block is not closed by *");
            }
            if !block.is_empty() {
                function_block(&mut st, &block, start_line);
            }
            pos = if end < chars.len() && chars[end] == '*' {
                end + 1
            } else {
                end
            };
        }
    }
    if !st.file.ended {
        let last = line;
        st.diag(last, Severity::Warning, "the file does not end with M02");
    }
    if st.in_region {
        let last = line;
        st.diag(
            last,
            Severity::Error,
            "a region (G36) is never closed by G37",
        );
    }
    st.file
}

fn parse_f64(text: &str) -> Option<f64> {
    let value: f64 = text.trim().parse().ok()?;
    value.is_finite().then_some(value)
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
            st.diag(line, Severity::Error, "an aperture macro (AM) has no name");
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
            "MM" => st.file.units = Some(Units::Millimetres),
            "IN" => st.file.units = Some(Units::Inches),
            _ => st.diag(
                line,
                Severity::Error,
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
        let mut parts = rest.split(',');
        let name = parts.next().unwrap_or("").to_owned();
        st.file.attributes.push(Attribute {
            kind: kind.to_owned(),
            name,
            values: parts.map(str::to_owned).collect(),
        });
    } else if ["LM", "LR", "LS", "AB"].iter().any(|k| cmd.starts_with(k)) {
        st.diag(
            line,
            Severity::Warning,
            format!("{} is not supported and was ignored", &cmd[..2]),
        );
    } else if ["IP", "OF", "SF", "AS", "IN", "LN", "MI", "IR", "KO", "PF"]
        .iter()
        .any(|k| cmd.starts_with(k))
    {
        st.diag(
            line,
            Severity::Info,
            format!("the deprecated command {} was ignored", &cmd[..2]),
        );
    } else {
        st.diag(
            line,
            Severity::Warning,
            format!(
                "unknown extended command '{}'",
                cmd.chars().take(12).collect::<String>()
            ),
        );
    }
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
                "FS: the zero omission must be L or T",
            );
            return;
        }
    };
    if zero == ZeroOmission::Trailing {
        st.diag(
            line,
            Severity::Warning,
            "FS: omitting trailing zeros is deprecated",
        );
    }
    match bytes.get(1) {
        Some('A') => {}
        Some('I') => st.diag(
            line,
            Severity::Error,
            "FS: incremental coordinates are not supported",
        ),
        _ => st.diag(
            line,
            Severity::Error,
            "FS: the coordinate mode must be A (absolute)",
        ),
    }
    let tail: String = bytes.iter().skip(2).collect();
    let (xs, ys) = match tail.split_once('Y') {
        Some((x, y)) if x.starts_with('X') => (&x[1..], y),
        _ => {
            st.diag(line, Severity::Error, "FS: expected X<digits>Y<digits>");
            return;
        }
    };
    let digits = |s: &str| -> Option<(u32, u32)> {
        if s.len() != 2 || !s.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let (i, d) = (s[..1].parse().ok()?, s[1..].parse().ok()?);
        (1..=6).contains(&i).then_some(())?;
        (d <= 6).then_some((i, d))
    };
    match (digits(xs), digits(ys)) {
        (Some(x), Some(y)) => {
            if x != y {
                st.diag(
                    line,
                    Severity::Warning,
                    "FS: X and Y use different formats; the X format is used for both",
                );
            }
            st.file.format = Some(CoordinateFormat {
                zero_omission: zero,
                integer_digits: x.0,
                decimal_digits: x.1,
            });
        }
        _ => st.diag(
            line,
            Severity::Error,
            "FS: the format must be two digits (1 to 6 integer, 0 to 6 decimal) for X and Y",
        ),
    }
}

fn aperture_definition(st: &mut State, rest: &str, line: usize) {
    let Some(rest) = rest.strip_prefix('D') else {
        st.diag(
            line,
            Severity::Error,
            "AD: the aperture number must start with D",
        );
        return;
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let Ok(number) = digits.parse::<u32>() else {
        st.diag(line, Severity::Error, "AD: missing aperture number");
        return;
    };
    if number < 10 {
        st.diag(
            line,
            Severity::Error,
            format!("AD: D{number} is reserved; apertures start at D10"),
        );
        return;
    }
    let spec = &rest[digits.len()..];
    let (template, params_text) = spec.split_once(',').unwrap_or((spec, ""));
    let mut params = Vec::new();
    for p in params_text.split('X').filter(|p| !p.is_empty()) {
        match parse_f64(p) {
            Some(v) => params.push(v),
            None => {
                st.diag(line, Severity::Error, format!("AD: '{p}' is not a number"));
                return;
            }
        }
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
                st.diag(line, Severity::Error, "AD: a polygon has 3 to 12 vertices");
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
                format!("AD: the {template} aperture has too few parameters"),
            );
            return;
        }
        name if !name.is_empty() => {
            if !st.file.macros.contains_key(name) {
                st.diag(
                    line,
                    Severity::Warning,
                    format!("AD: the macro '{name}' is not defined before its use"),
                );
            }
            Aperture::Macro {
                name: name.to_owned(),
                params,
            }
        }
        _ => {
            st.diag(line, Severity::Error, "AD: missing aperture template");
            return;
        }
    };
    if aperture_has_bad_size(&aperture) {
        st.diag(
            line,
            Severity::Warning,
            format!("AD: D{number} has a negative size"),
        );
    }
    if st.file.apertures.insert(number, aperture).is_some() {
        st.diag(
            line,
            Severity::Warning,
            format!("AD: D{number} is defined twice; the last definition wins"),
        );
    }
}

fn aperture_has_bad_size(a: &Aperture) -> bool {
    match a {
        Aperture::Circle { diameter, .. } | Aperture::Polygon { diameter, .. } => *diameter < 0.0,
        Aperture::Rectangle { x, y, .. } | Aperture::Obround { x, y, .. } => *x < 0.0 || *y < 0.0,
        Aperture::Macro { .. } => false,
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
                    format!("SR: bad value '{letter}{value}'"),
                );
                return;
            }
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
        _ => st.diag(line, Severity::Error, "SR: needs X, Y, I and J"),
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
    if block.starts_with("G04") {
        return;
    }
    let words = words(block);
    if words.is_empty() {
        st.diag(
            line,
            Severity::Warning,
            "a block has no letters and was ignored",
        );
        return;
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
                    "M00/M01 are deprecated and were ignored",
                ),
                _ => st.diag(line, Severity::Warning, format!("unknown M{digits}")),
            },
            'X' | 'Y' | 'I' | 'J' => match coordinate(st, digits, line) {
                Some(v) => match *letter {
                    'X' => x = Some(v),
                    'Y' => y = Some(v),
                    'I' => i = Some(v),
                    _ => j = Some(v),
                },
                None => return,
            },
            'D' => match digits.parse::<u32>() {
                Ok(n) => d = Some(n),
                Err(_) => {
                    st.diag(
                        line,
                        Severity::Error,
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
                    format!("unknown word '{other}{digits}' was ignored"),
                );
            }
        }
    }
    if let Some(v) = x {
        st.x = Some(v);
    }
    if let Some(v) = y {
        st.y = Some(v);
    }
    let Some(d) = d else {
        if x.is_some() || y.is_some() {
            st.diag(line, Severity::Warning, "coordinates without D01, D02 or D03 (the deprecated implied operation) were ignored");
        }
        return;
    };
    match d {
        1..=3 => {
            let (Some(cx), Some(cy)) = (st.x, st.y) else {
                st.diag(
                    line,
                    Severity::Error,
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
                            "a flash (D03) happens before any aperture is selected",
                        );
                    } else if !st.file.apertures.contains_key(&st.aperture.unwrap_or(0)) {
                        st.diag(
                            line,
                            Severity::Error,
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
                        "a draw (D01) happens before any aperture is selected",
                    );
                } else if !st.file.apertures.contains_key(&st.aperture.unwrap_or(0)) {
                    st.diag(
                        line,
                        Severity::Error,
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
                    format!("D{n} is selected but not defined (yet)"),
                );
            }
            st.push(line, Operation::SelectAperture(n));
        }
        n => st.diag(
            line,
            Severity::Error,
            format!("D{n:02} is not a valid operation or aperture"),
        ),
    }
}

/// The point a draw starts from: the point before this block's coordinates were applied.
fn current_point(st: &mut State, line: usize, _words: &[(char, String)]) -> Option<Point> {
    // The previous operation that carried a coordinate left its end point in `last_end`.
    let last = st.file.commands.iter().rev().find_map(|c| match &c.op {
        Operation::Move { to } => Some(*to),
        Operation::Flash { at } => Some(*at),
        Operation::Line { to, .. } | Operation::Arc { to, .. } => Some(*to),
        _ => None,
    });
    if last.is_none() {
        st.diag(
            line,
            Severity::Error,
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
                st.diag(line, Severity::Error, "G36 inside a region");
            }
            st.in_region = true;
            st.push(line, Operation::RegionStart);
        }
        Ok(37) => {
            if !st.in_region {
                st.diag(line, Severity::Error, "G37 without G36");
            }
            st.in_region = false;
            st.push(line, Operation::RegionEnd);
        }
        Ok(74) => {
            st.quadrant = QuadrantMode::Single;
            st.diag(
                line,
                Severity::Warning,
                "G74 (single quadrant) is deprecated",
            );
        }
        Ok(75) => st.quadrant = QuadrantMode::Multi,
        Ok(70) => {
            st.file.units = Some(Units::Inches);
            st.diag(line, Severity::Warning, "G70 is deprecated: use MOIN");
        }
        Ok(71) => {
            st.file.units = Some(Units::Millimetres);
            st.diag(line, Severity::Warning, "G71 is deprecated: use MOMM");
        }
        Ok(54) | Ok(55) => st.diag(
            line,
            Severity::Info,
            "G54/G55 are deprecated and were ignored",
        ),
        Ok(90) => {}
        Ok(91) => st.diag(
            line,
            Severity::Error,
            "G91 (incremental coordinates) is not supported",
        ),
        _ => st.diag(line, Severity::Warning, format!("unknown G{digits}")),
    }
}

fn coordinate(st: &mut State, digits: &str, line: usize) -> Option<f64> {
    let Some(format) = st.file.format else {
        st.diag(
            line,
            Severity::Error,
            "a coordinate appears before the format (FS) is given",
        );
        return None;
    };
    match decode_coordinate(digits, format) {
        Some(v) => Some(v),
        None => {
            st.diag(
                line,
                Severity::Error,
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
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    if digits.is_empty() {
        return None;
    }
    let magnitude = if digits.contains('.') {
        parse_f64(digits)?
    } else {
        if !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let width = (format.integer_digits + format.decimal_digits) as usize;
        // More digits than the format has cannot be a coordinate of this file: refuse it instead of returning an absurd value.
        if digits.len() > width {
            return None;
        }
        let padded: String = match format.zero_omission {
            ZeroOmission::Leading => digits.to_owned(),
            ZeroOmission::Trailing => format!("{digits:0<width$}"),
        };
        let integer: f64 = padded.parse().ok()?;
        integer / 10f64.powi(format.decimal_digits as i32)
    };
    Some(if negative { -magnitude } else { magnitude })
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
