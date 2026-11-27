//! # eh3d-parser-excellon
//!
//! A tolerant reader for Excellon (NC drill) files: tools, drill hits, slots (`G85`) and simple
//! routes (`M15` / `G01` / `M16`).
//!
//! [`parse`] never panics and never stops at the first problem: what it cannot understand becomes
//! a [`Diagnostic`] with its line number and the reading goes on.
//!
//! Zero suppression follows the common reading of the two words: `TZ` means trailing zeros are
//! present and leading zeros are suppressed ([`ZeroSuppression::Leading`]); `LZ` means leading
//! zeros are present and trailing zeros are suppressed ([`ZeroSuppression::Trailing`]).
//! Coordinates with a decimal point are always read as they are.
//!
//! A coordinate without a point of a file that declares no format (`FILE_FORMAT`, a number mask) or no zeros is
//! still read, as 2.4 inches or 3.3 millimetres with the leading zeros left out, and the supposition is told once
//! (`format-assumed`, `zeros-assumed`). A program that must not guess a position calls
//! [`ExcellonFile::refuse_assumptions`].
//!
//! Also read: arcs in routes (`G02`/`G03` with a radius `A` or the offsets `I`/`J`), incremental coordinates
//! (`G91`, `ICI,ON`), the repeat `R<n>X<step>Y<step>`, the attributes of X2 written as `; #@! TF...` comments,
//! the `Holesize` comments of Allegro (plating, quantity, tolerance of each tool), a decimal comma and the program
//! that wrote the file. A limit that is reached ([`ExcellonLimits`]) stops the reading cleanly.
//!
//! Not done: the pattern offset `M02X..` and any drawing of geometry.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::string_slice)]

use std::collections::BTreeMap;

pub mod snapshot;

pub use eh3d_parser_common::dialect::Dialect;
pub use eh3d_parser_common::{
    codes, decode_coordinate, Diagnostic, Severity, Units, ZeroSuppression,
};
use eh3d_parser_common::{coordinate_error_code, decode_input, parse_f64, resolve_units};
pub use eh3d_parser_common::{read_file_limited, InputError, InputLimits, ReadError};

/// A file with no units whose holes span less than this many units may be in inches (no board is 5 mm wide).
const SUSPECT_SPAN: f64 = 5.0;

/// The most diagnostics kept for one file: a damaged file of millions of bad lines does not fill the memory with
/// messages. The notices of what was supposed and the limit that stopped the reading are kept past it.
pub const MAX_DIAGNOSTICS: usize = 100_000;

/// A point in the units of the file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// X.
    pub x: f64,
    /// Y.
    pub y: f64,
}

/// A drill (or router) bit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tool {
    /// The tool number.
    pub number: u32,
    /// The diameter in the units of the file.
    pub diameter: f64,
    /// The feed rate (`F`), when the file gives it.
    pub feed: Option<f64>,
    /// The spindle speed (`S`), when the file gives it.
    pub speed: Option<f64>,
}

/// A hole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// The line in the file.
    pub line: usize,
    /// The tool.
    pub tool: u32,
    /// Where.
    pub at: Point,
}

/// A slot drilled with `G85`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// The line in the file.
    pub line: usize,
    /// The tool.
    pub tool: u32,
    /// One end.
    pub from: Point,
    /// The other end.
    pub to: Point,
}

/// An arc of a route (`G02` clockwise, `G03` counterclockwise): the piece of the path that ends at
/// `points[to_index]` is this arc instead of a straight line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteArc {
    /// The index in [`Route::points`] of the end of the arc (its start is the point before).
    pub to_index: usize,
    /// True for `G02`.
    pub clockwise: bool,
    /// The radius (`A`), when the file gives it.
    pub radius: Option<f64>,
    /// The offsets `(I, J)` from the start to the centre, when the file gives them.
    pub center_offset: Option<(f64, f64)>,
}

/// A routed path: tool down at the first point, straight segments to the others.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// The line where the tool went down.
    pub line: usize,
    /// The tool.
    pub tool: Option<u32>,
    /// The points of the path.
    pub points: Vec<Point>,
    /// The pieces of the path that are arcs; all the others are straight.
    pub arcs: Vec<RouteArc>,
}

/// An attribute of X2 written as a comment (`; #@! TF.FileFunction,Plated,1,4,PTH`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// `TF`, `TA`, `TO` or `TD`.
    pub kind: String,
    /// The name, for example `.FileFunction`.
    pub name: String,
    /// The values.
    pub values: Vec<String>,
}

/// What a `Holesize` comment of Allegro (and the programs that write alike) says of a tool.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ToolNote {
    /// True for plated holes, false for holes that are not, when the comment says it.
    pub plated: Option<bool>,
    /// How many holes the tool makes, when the comment says it.
    pub quantity: Option<u32>,
    /// The tolerance towards a bigger hole.
    pub tolerance_plus: Option<f64>,
    /// The tolerance towards a smaller hole, as a positive number.
    pub tolerance_minus: Option<f64>,
}

/// Whether the holes of a file are plated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plating {
    /// Plated through holes.
    Plated,
    /// Holes that are not plated.
    NonPlated,
}

/// The limits of one reading: past a limit the reading stops with a `limit-exceeded` error and returns what it
/// has read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcellonLimits {
    /// The most holes, slots and route points together.
    pub max_items: usize,
    /// The most tools.
    pub max_tools: usize,
    /// The most holes one repeat (`R<n>`) may make.
    pub max_repeat: u32,
    /// The most attributes.
    pub max_attributes: usize,
}

impl Default for ExcellonLimits {
    fn default() -> Self {
        ExcellonLimits {
            max_items: 20_000_000,
            max_tools: 100_000,
            max_repeat: 100_000,
            max_attributes: 100_000,
        }
    }
}

/// Everything read from an Excellon file.
#[derive(Debug, Clone, PartialEq)]
pub struct ExcellonFile {
    /// The units, when the file says them.
    pub units: Option<Units>,
    /// The zero suppression, when the file says it.
    pub zeros: Option<ZeroSuppression>,
    /// The tools by number.
    pub tools: BTreeMap<u32, Tool>,
    /// The holes in order.
    pub drills: Vec<Hit>,
    /// The slots in order.
    pub slots: Vec<Slot>,
    /// The routed paths in order.
    pub routes: Vec<Route>,
    /// The attributes of X2 written as comments.
    pub attributes: Vec<Attribute>,
    /// What the `Holesize` comments say of each tool.
    pub notes: BTreeMap<u32, ToolNote>,
    /// What a comment `TYPE=PLATED` or `TYPE=NON_PLATED` (Altium and others) says of the whole file.
    pub declared_plating: Option<Plating>,
    /// The program that seems to have written the file.
    pub dialect: Dialect,
    /// Whether the file ends with `M30`.
    pub ended: bool,
    /// What the reader noticed.
    pub diagnostics: Vec<Diagnostic>,
}

impl ExcellonFile {
    /// The units of the file, or millimetres (with the `units-missing` warning in the
    /// diagnostics) when it does not declare any.
    pub fn effective_units(&self) -> Units {
        self.units.unwrap_or(eh3d_parser_common::ASSUMED_UNITS)
    }

    /// True when the file has no diagnostic of severity `Error`.
    pub fn is_clean(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|d| d.severity < Severity::Error)
    }

    /// The notices of what the reader had to suppose to place the holes: the coordinate format
    /// (`format-assumed`), the zero suppression (`zeros-assumed`) and, when the file has holes, slots or routes,
    /// the units (`units-missing`). Empty when every position follows from what the file or the caller says.
    pub fn assumptions(&self) -> impl Iterator<Item = &Diagnostic> {
        let drawn = self.has_positions();
        self.diagnostics
            .iter()
            .filter(move |d| is_assumption(d.code, drawn))
    }

    /// Turns every notice of [`ExcellonFile::assumptions`] into an error, so that [`ExcellonFile::is_clean`]
    /// is false for a file whose positions rest on a supposition. For a program that moves a machine with what it
    /// reads. Returns true when there was at least one. The holes are not changed.
    pub fn refuse_assumptions(&mut self) -> bool {
        let drawn = self.has_positions();
        let mut found = false;
        for d in &mut self.diagnostics {
            if is_assumption(d.code, drawn) {
                d.severity = Severity::Error;
                found = true;
            }
        }
        found
    }

    fn has_positions(&self) -> bool {
        !(self.drills.is_empty() && self.slots.is_empty() && self.routes.is_empty())
    }

    /// Whether the holes are plated, from the attribute `.FileFunction` (`Plated`, `NonPlated`) or, when there is
    /// none, from the `Holesize` comments if they all say the same. `None` when the file does not say it.
    pub fn plating(&self) -> Option<Plating> {
        for a in self
            .attributes
            .iter()
            .filter(|a| a.kind == "TF" && a.name == ".FileFunction")
        {
            match a.values.first().map(String::as_str) {
                Some("Plated") => return Some(Plating::Plated),
                Some("NonPlated") => return Some(Plating::NonPlated),
                _ => {}
            }
        }
        if self.declared_plating.is_some() {
            return self.declared_plating;
        }
        let mut seen = self.notes.values().filter_map(|n| n.plated);
        let first = seen.next()?;
        seen.all(|p| p == first).then_some(if first {
            Plating::Plated
        } else {
            Plating::NonPlated
        })
    }
}

fn is_assumption(code: &str, drawn: bool) -> bool {
    code == codes::FORMAT_ASSUMED
        || code == codes::ZEROS_ASSUMED
        || (drawn && code == codes::UNITS_MISSING)
}

struct State {
    file: ExcellonFile,
    format: Option<(u32, u32)>,
    /// The units the file itself has declared (not those the caller supposes).
    declared: Option<Units>,
    format_told: bool,
    zeros_told: bool,
    diagnostics_cut: bool,
    header: bool,
    tool: Option<u32>,
    x: f64,
    y: f64,
    routing: bool,
    route_down: bool,
    current_route: Option<Route>,
    /// The motion of a route: straight, `G02` or `G03`.
    mode: RouteMode,
    /// Coordinates are increments of the last position (`G91`, `ICI,ON`).
    incremental: bool,
    limits: ExcellonLimits,
    items: usize,
    stopped: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RouteMode {
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
                self.keep(
                    line,
                    Severity::Error,
                    codes::TOO_MANY_DIAGNOSTICS,
                    format!("more than {MAX_DIAGNOSTICS} diagnostics: the rest are not kept"),
                );
            }
            return;
        }
        self.keep(line, severity, code, message);
    }

    /// A diagnostic that is kept whatever the number there are already (each one is given once).
    fn keep(
        &mut self,
        line: usize,
        severity: Severity,
        code: &'static str,
        message: impl Into<String>,
    ) {
        self.file
            .diagnostics
            .push(Diagnostic::new(line, severity, code, message));
    }

    fn declare_units(&mut self, units: Units, line: usize) {
        if self.declared.is_some_and(|before| before != units) {
            self.diag(
                line,
                Severity::Error,
                codes::UNITS_CONFLICT,
                "the file declares other units than before: the last ones are used",
            );
        }
        self.declared = Some(units);
        self.file.units = Some(units);
    }

    /// Counts one more hole, slot or route point; false (and the reading stops) when the limit is reached.
    fn count_item(&mut self, line: usize) -> bool {
        if self.items >= self.limits.max_items {
            self.limit_hit(line, "more holes, slots and route points than the limit");
            return false;
        }
        self.items += 1;
        true
    }

    fn limit_hit(&mut self, line: usize, what: &str) {
        if !self.stopped {
            self.stopped = true;
            self.keep(
                line,
                Severity::Error,
                codes::LIMIT_EXCEEDED,
                format!("{what}: the reading stops here"),
            );
        }
    }

    fn finish_route(&mut self) {
        if let Some(route) = self.current_route.take() {
            self.file.routes.push(route);
        }
    }
}

/// Reads a file from bytes: UTF-8 (with or without a byte order mark) or, with a warning
/// (`input-not-utf8`), Latin-1. An input above the default limit of 256 MiB is refused.
pub fn parse_bytes(bytes: &[u8]) -> Result<ExcellonFile, InputError> {
    parse_bytes_with(bytes, &InputLimits::default())
}

/// Like [`parse_bytes`] with a limit of your own.
pub fn parse_bytes_with(bytes: &[u8], limits: &InputLimits) -> Result<ExcellonFile, InputError> {
    let input = decode_input(bytes, limits)?;
    let mut file = parse(&input.text);
    file.diagnostics.splice(0..0, input.diagnostics);
    Ok(file)
}

/// Reads an Excellon file whose units the caller knows: the units given win over the ones the file declares (or does not declare).
/// The notices about missing or suspect units are dropped, and a file that declares other units gets a warning `units-forced`.
pub fn parse_with_units(text: &str, units: Units) -> ExcellonFile {
    let mut file = parse(text);
    if file.units.is_none() {
        // the format that is supposed for a file without one follows the units: 2.4 in inches, 3.3 in millimetres
        file = parse_with_format(
            text,
            &ExcellonDefaults {
                default_units: Some(units),
                ..ExcellonDefaults::default()
            },
        );
    }
    if let Some(declared) = file.units {
        if declared != units {
            file.diagnostics.push(Diagnostic::new(
                1,
                Severity::Warning,
                codes::UNITS_FORCED,
                "the file declares other units than the ones given: the ones given are used",
            ));
        }
    }
    file.diagnostics
        .retain(|d| d.code != codes::UNITS_MISSING && d.code != codes::UNITS_SUSPECT);
    file.units = Some(units);
    file
}

/// What to assume about a file that does not say it itself: the coordinate format, the zero
/// suppression and the units.
///
/// Every field is optional (`None` assumes nothing, which is what [`parse`] does). A value is used
/// only when the file does not declare the same thing: `FILE_FORMAT=i:d`, `INCH,TZ,00.0000`,
/// `METRIC`, `M71` or `M72` in the file win over it, wherever they are. Write it with
/// `ExcellonDefaults { default_format: Some((2, 4)), ..Default::default() }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExcellonDefaults {
    /// The digits before and after the point of a coordinate written without one, as
    /// `(integer, decimal)`: the integer digits go from 1 to 8 and the decimal ones from 0 to 8,
    /// as in the `FILE_FORMAT` comment. Anything else is ignored with a `defaults-invalid` warning.
    pub default_format: Option<(u8, u8)>,
    /// The zero suppression of a file that does not say it.
    pub default_zeros: Option<ZeroSuppression>,
    /// The units of a file that does not say them (the `units-missing` and `units-suspect`
    /// notices are then not given, and [`ExcellonFile::units`] is these units).
    pub default_units: Option<Units>,
}

impl ExcellonDefaults {
    /// True when `default_format` is empty or inside the limits the reader accepts in a file.
    pub fn is_valid(&self) -> bool {
        match self.default_format {
            Some((i, d)) => (1..=8).contains(&i) && d <= 8,
            None => true,
        }
    }
}

/// Reads an Excellon file whose format, zeros and units are assumed when the file does not declare
/// them (see [`ExcellonDefaults`]). What the file declares always wins, the lines of the
/// diagnostics are those of the text, and `ExcellonDefaults::default()` reads exactly as [`parse`].
/// A format of the defaults that is out of range is ignored and told in a `defaults-invalid`
/// warning at line 1.
pub fn parse_with_format(text: &str, defaults: &ExcellonDefaults) -> ExcellonFile {
    parse_inner(text, defaults, &ExcellonLimits::default())
}

/// Like [`parse`] with the limits given: past a limit the reading stops with a `limit-exceeded` error.
pub fn parse_with_limits(text: &str, limits: &ExcellonLimits) -> ExcellonFile {
    parse_inner(text, &ExcellonDefaults::default(), limits)
}

/// Like [`parse_bytes`] with the assumptions of [`parse_with_format`].
pub fn parse_bytes_with_format(
    bytes: &[u8],
    defaults: &ExcellonDefaults,
) -> Result<ExcellonFile, InputError> {
    let input = decode_input(bytes, &InputLimits::default())?;
    let mut file = parse_inner(&input.text, defaults, &ExcellonLimits::default());
    file.diagnostics.splice(0..0, input.diagnostics);
    Ok(file)
}

/// Reads an Excellon file. Never fails: problems are in [`ExcellonFile::diagnostics`].
pub fn parse(text: &str) -> ExcellonFile {
    parse_inner(
        text,
        &ExcellonDefaults::default(),
        &ExcellonLimits::default(),
    )
}

fn parse_inner(text: &str, defaults: &ExcellonDefaults, limits: &ExcellonLimits) -> ExcellonFile {
    let format = if defaults.is_valid() {
        defaults
            .default_format
            .map(|(i, d)| (u32::from(i), u32::from(d)))
    } else {
        None
    };
    let mut st = State {
        file: ExcellonFile {
            units: defaults.default_units,
            zeros: defaults.default_zeros,
            tools: BTreeMap::new(),
            drills: Vec::new(),
            slots: Vec::new(),
            routes: Vec::new(),
            attributes: Vec::new(),
            notes: BTreeMap::new(),
            declared_plating: None,
            dialect: eh3d_parser_common::dialect::detect(text),
            ended: false,
            diagnostics: Vec::new(),
        },
        format,
        declared: None,
        format_told: false,
        zeros_told: false,
        diagnostics_cut: false,
        mode: RouteMode::Linear,
        incremental: false,
        limits: limits.clone(),
        items: 0,
        stopped: false,
        header: false,
        tool: None,
        x: 0.0,
        y: 0.0,
        routing: false,
        route_down: false,
        current_route: None,
    };
    if !defaults.is_valid() {
        st.diag(
            1,
            Severity::Warning,
            codes::DEFAULTS_INVALID,
            "the default format is out of range (1..=8 integer digits, 0..=8 decimal digits) and was ignored",
        );
    }
    let mut started = false;
    for (index, raw) in text.lines().enumerate() {
        if st.stopped {
            break;
        }
        let line = index + 1;
        let trimmed = raw.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() {
            continue;
        }
        if let Some(comment) = trimmed.strip_prefix(';') {
            comment_line(&mut st, comment, line);
            continue;
        }
        if st.file.ended {
            st.diag(
                line,
                Severity::Warning,
                codes::TEXT_AFTER_END,
                "text after M30 was ignored",
            );
            continue;
        }
        let upper = trimmed.to_ascii_uppercase();
        if upper == "M48" {
            st.header = true;
            started = true;
        } else if upper == "%" || upper == "M95" {
            st.header = false;
            started = true;
        } else if st.header {
            header_line(&mut st, &upper, line);
        } else {
            body_line(&mut st, &upper, line);
        }
    }
    st.finish_route();
    resolve_units(st.file.units, 1, &mut st.file.diagnostics);
    if st.file.units.is_none() && st.file.drills.len() >= 2 {
        // a drill file in inches without its header (old Eagle or Protel files) read as millimetres is 25.4 times too small
        let (mut lo, mut hi) = (
            (f64::INFINITY, f64::INFINITY),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for h in &st.file.drills {
            lo = (lo.0.min(h.at.x), lo.1.min(h.at.y));
            hi = (hi.0.max(h.at.x), hi.1.max(h.at.y));
        }
        if (hi.0 - lo.0).max(hi.1 - lo.1) < SUSPECT_SPAN {
            st.diag(
                1,
                Severity::Warning,
                codes::UNITS_SUSPECT,
                "the file declares no units and its holes span less than 5 units: it may be in inches",
            );
        }
    }
    if !started {
        st.diag(
            1,
            Severity::Warning,
            codes::HEADER_MISSING,
            "the file has no M48 header",
        );
    }
    if !st.file.ended {
        let last = text.lines().count().max(1);
        st.diag(
            last,
            Severity::Warning,
            codes::END_MISSING,
            "the file does not end with M30",
        );
    }
    st.file
}

/// `Holesize 1 = 0.600000 Tolerance = +0.100000/-0.100000 PLATED MM Quantity = 12` (Allegro).
fn holesize_note(st: &mut State, comment: &str) {
    let lower = comment.trim().to_ascii_lowercase();
    let Some(rest) = lower.strip_prefix("holesize") else {
        return;
    };
    let number: String = rest
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let Ok(tool) = number.parse::<u32>() else {
        return;
    };
    let mut note = ToolNote::default();
    let words: Vec<&str> = rest.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        match *w {
            "plated" => note.plated = Some(true),
            "non_plated" | "non-plated" | "nonplated" | "unplated" => note.plated = Some(false),
            "quantity" => {
                note.quantity = words
                    .get(i + 1)
                    .map(|v| v.trim_start_matches('='))
                    .filter(|v| !v.is_empty())
                    .or_else(|| words.get(i + 2).copied())
                    .and_then(|v| v.parse().ok());
            }
            "tolerance" => {
                // up to the word that holds the slash: `= +0.1/-0.1`, `=+0.1/-0.1` or `+0.1/-0.1`
                let mut tail = String::new();
                for word in words.iter().skip(i + 1).take(3) {
                    tail.push_str(word);
                    if word.contains('/') {
                        break;
                    }
                }
                if let Some((plus, minus)) = tail.trim_start_matches('=').split_once('/') {
                    note.tolerance_plus = parse_f64(plus.trim_start_matches('+'));
                    note.tolerance_minus = parse_f64(minus.trim_start_matches('-'));
                }
            }
            _ => {}
        }
    }
    if note != ToolNote::default() && st.file.notes.len() < st.limits.max_tools {
        st.file.notes.insert(tool, note);
    }
}

fn comment_line(st: &mut State, comment: &str, line: usize) {
    holesize_note(st, comment);
    let type_word = comment.trim().to_ascii_uppercase().replace(' ', "");
    if let Some(kind) = type_word.strip_prefix("TYPE=") {
        match kind {
            "PLATED" => st.file.declared_plating = Some(Plating::Plated),
            "NON_PLATED" | "NON-PLATED" | "NONPLATED" | "UNPLATED" => {
                st.file.declared_plating = Some(Plating::NonPlated)
            }
            _ => {}
        }
    }
    // X2 attributes written as comments: `; #@! TF.FileFunction,Plated,1,4,PTH`
    if let Some(rest) = comment.trim().strip_prefix("#@!") {
        let rest: String = rest.chars().filter(|c| !c.is_whitespace()).collect();
        if let Some((kind, rest)) = ["TF", "TA", "TO", "TD"]
            .iter()
            .find_map(|k| rest.strip_prefix(k).map(|r| (*k, r.to_owned())))
        {
            if st.file.attributes.len() >= st.limits.max_attributes {
                st.limit_hit(line, "more attributes than the limit");
                return;
            }
            let mut parts = rest.split(',');
            let name = parts.next().unwrap_or("").to_owned();
            st.file.attributes.push(Attribute {
                kind: kind.to_owned(),
                name,
                values: parts.map(str::to_owned).collect(),
            });
        }
    }
    // KiCad and others state the integer format in a comment: FILE_FORMAT=4:3
    let compact: String = comment.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(rest) = compact.to_ascii_uppercase().strip_prefix("FILE_FORMAT=") {
        let digits = rest
            .split_once(':')
            .and_then(|(i, d)| Some((i.parse::<u32>().ok()?, d.parse::<u32>().ok()?)));
        match digits {
            Some((i, d)) if (1..=8).contains(&i) && d <= 8 => st.format = Some((i, d)),
            _ => st.diag(
                line,
                Severity::Warning,
                codes::FORMAT_COMMENT_INVALID,
                "FILE_FORMAT comment is not understood",
            ),
        }
    }
}

fn header_line(st: &mut State, upper: &str, line: usize) {
    let (word, rest) = upper.split_once(',').unwrap_or((upper, ""));
    let (word, rest) = (word.trim(), rest.trim());
    match word {
        // the old way of saying the units: M71 millimetres, M72 inches
        "M71" | "M72" => st.declare_units(
            if word == "M71" {
                Units::Millimetres
            } else {
                Units::Inches
            },
            line,
        ),
        "METRIC" | "INCH" => {
            st.declare_units(
                if word == "METRIC" {
                    Units::Millimetres
                } else {
                    Units::Inches
                },
                line,
            );
            for part in rest.split(',').map(str::trim).filter(|p| !p.is_empty()) {
                match part {
                    "TZ" => st.file.zeros = Some(ZeroSuppression::Leading),
                    "LZ" => st.file.zeros = Some(ZeroSuppression::Trailing),
                    other if other.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                        // 000.000 or 00.0000: the number of digits before and after the point
                        // the same limits as the format comment: a mask of hundreds of digits would make every coordinate zero
                        match other.split_once('.') {
                            Some((i, d)) if (1..=8).contains(&i.len()) && d.len() <= 8 => {
                                st.format = Some((i.len() as u32, d.len() as u32));
                            }
                            _ => st.diag(
                                line,
                                Severity::Warning,
                                codes::FORMAT_COMMENT_INVALID,
                                "the number format of the header is not understood",
                            ),
                        }
                    }
                    other => st.diag(
                        line,
                        Severity::Warning,
                        codes::OPTION_UNKNOWN,
                        format!("unknown option '{other}' after {word}"),
                    ),
                }
            }
        }
        "FMAT" | "VER" | "DETECT" | "ATC" | "ICI" if word == "ICI" => match rest {
            "OFF" => st.incremental = false,
            "ON" => {
                st.incremental = true;
                st.diag(
                    line,
                    Severity::Warning,
                    codes::DEPRECATED_COMMAND,
                    "ICI,ON: incremental coordinates are added to the last position",
                );
            }
            _ => st.diag(
                line,
                Severity::Error,
                codes::INCREMENTAL_UNSUPPORTED,
                "ICI has a value that is not ON or OFF",
            ),
        },
        "FMAT" | "VER" | "DETECT" | "ATC" => st.diag(
            line,
            Severity::Info,
            codes::COMMAND_IGNORED,
            format!("{word} was ignored"),
        ),
        _ if upper.starts_with('T') => tool_definition(st, upper, line),
        _ if starts_with_a_position(upper) => st.diag(
            line,
            Severity::Error,
            codes::UNKNOWN_HEADER_LINE,
            "a coordinate inside the header is not drilled: the header is not closed by % or M95",
        ),
        _ => st.diag(
            line,
            Severity::Warning,
            codes::UNKNOWN_HEADER_LINE,
            format!(
                "unknown header line '{}'",
                upper.chars().take(20).collect::<String>()
            ),
        ),
    }
}

/// True for a line that starts like a coordinate: `X` or `Y` and then a digit, a sign or a point.
fn starts_with_a_position(upper: &str) -> bool {
    let mut chars = upper.chars();
    matches!(chars.next(), Some('X' | 'Y'))
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.'))
}

fn tool_definition(st: &mut State, upper: &str, line: usize) {
    let replaced;
    let mut upper = upper;
    if upper.contains(',') {
        // a computer set to a language that writes the decimal point as a comma wrote some files that way
        replaced = upper.replace(',', ".");
        st.diag(
            line,
            Severity::Warning,
            codes::DECIMAL_COMMA,
            "a tool definition uses a comma as the decimal point; it was read as a point",
        );
        upper = &replaced;
    }
    let after_t = upper.strip_prefix('T').unwrap_or(upper);
    let digits: String = after_t.chars().take_while(char::is_ascii_digit).collect();
    let Ok(number) = digits.parse::<u32>() else {
        st.diag(
            line,
            Severity::Error,
            codes::TOOL_NUMBER_MISSING,
            "a tool definition has no tool number",
        );
        return;
    };
    let mut rest = after_t.get(digits.len()..).unwrap_or("");
    let mut tool = Tool {
        number,
        diameter: 0.0,
        feed: None,
        speed: None,
    };
    let mut have_diameter = false;
    while let Some(letter) = rest.chars().next() {
        let after_letter = rest.get(letter.len_utf8()..).unwrap_or("");
        let number_text: String = after_letter
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
            .collect();
        match (letter, parse_f64(&number_text)) {
            ('C', Some(v)) if v > 0.0 => {
                tool.diameter = v;
                have_diameter = true;
            }
            ('F', Some(v)) => tool.feed = Some(v),
            ('S', Some(v)) => tool.speed = Some(v),
            ('B' | 'H' | 'Z', _) => {}
            ('C', _) => {
                st.diag(
                    line,
                    Severity::Error,
                    codes::TOOL_DIAMETER_INVALID,
                    "a tool has a diameter that is not a positive number",
                );
                return;
            }
            (other, _) => {
                st.diag(
                    line,
                    Severity::Warning,
                    codes::TOOL_PARAMETER_UNKNOWN,
                    format!("unknown tool parameter '{other}' was ignored"),
                );
            }
        }
        rest = after_letter.get(number_text.len()..).unwrap_or("");
    }
    if !have_diameter {
        st.diag(
            line,
            Severity::Error,
            codes::TOOL_DIAMETER_MISSING,
            format!("tool T{number} is defined without a diameter"),
        );
        return;
    }
    if st.file.tools.len() >= st.limits.max_tools && !st.file.tools.contains_key(&number) {
        st.limit_hit(line, "more tools than the limit");
        return;
    }
    if st.file.tools.insert(number, tool).is_some() {
        st.diag(
            line,
            Severity::Warning,
            codes::TOOL_REDEFINED,
            format!("tool T{number} is defined twice; the last definition wins"),
        );
    }
}

fn body_line(st: &mut State, upper: &str, line: usize) {
    if let Some((before, after)) = upper.split_once("G85") {
        slot(st, before, after, line);
        return;
    }
    // A line is a series of words such as T01, G05, X1.5Y2, M30.
    let mut words: Vec<(char, String)> = Vec::new();
    for c in upper.chars().filter(|c| !c.is_whitespace()) {
        if c.is_ascii_alphabetic() {
            words.push((c, String::new()));
        } else if let Some(last) = words.last_mut() {
            last.1.push(c);
        }
    }
    if words.is_empty() {
        st.diag(
            line,
            Severity::Warning,
            codes::COMMAND_IGNORED,
            format!(
                "'{}' was ignored",
                upper.chars().take(20).collect::<String>()
            ),
        );
        return;
    }
    // what stands before the first letter belongs to no word: a line that lost its X would drill somewhere else
    let lead: String = upper
        .chars()
        .take_while(|c| !c.is_ascii_alphabetic())
        .filter(|c| !c.is_whitespace())
        .collect();
    if !lead.is_empty() {
        let severity = if lead.chars().any(|c| c.is_ascii_digit()) {
            Severity::Error
        } else {
            Severity::Warning
        };
        st.diag(
            line,
            severity,
            codes::CHARACTER_IGNORED,
            format!(
                "'{}' before the first letter of the line was left out",
                lead.chars().take(20).collect::<String>()
            ),
        );
    }
    let (mut x, mut y) = (None, None);
    let (mut radius, mut arc_i, mut arc_j) = (None, None, None);
    let mut g: Option<u32> = None;
    let (int_digits, dec_digits) = current_format(st);
    let mut comma_told = false;
    for (letter, digits) in &words {
        match *letter {
            'T' => match digits.parse::<u32>() {
                Ok(0) => st.tool = None,
                Ok(n) => {
                    if !st.file.tools.contains_key(&n) {
                        st.diag(
                            line,
                            Severity::Error,
                            codes::TOOL_UNDEFINED,
                            format!("T{n} is selected but never defined"),
                        );
                    }
                    st.tool = Some(n);
                }
                Err(_) => st.diag(
                    line,
                    Severity::Error,
                    codes::TOOL_SELECTION_INVALID,
                    "a tool selection has no number",
                ),
            },
            'G' => match digits.parse::<u32>() {
                Ok(n @ (0 | 1)) => {
                    g = Some(n);
                    st.routing = true;
                    st.mode = RouteMode::Linear;
                }
                Ok(n @ (2 | 3)) => {
                    g = Some(n);
                    st.routing = true;
                    st.mode = if n == 2 {
                        RouteMode::Clockwise
                    } else {
                        RouteMode::CounterClockwise
                    };
                }
                Ok(5) => {
                    st.routing = false;
                    st.mode = RouteMode::Linear;
                    st.finish_route();
                }
                Ok(90) => st.incremental = false,
                Ok(91) => {
                    st.incremental = true;
                    st.diag(
                        line,
                        Severity::Warning,
                        codes::DEPRECATED_COMMAND,
                        "G91 (incremental coordinates) is deprecated; each X and Y is added to the last position",
                    );
                }
                // a zero set: judged after the words of the line are read
                Ok(92 | 93) => {}
                _ => st.diag(
                    line,
                    Severity::Warning,
                    codes::UNKNOWN_G_CODE,
                    format!("unknown G{digits}"),
                ),
            },
            'M' => match digits.parse::<u32>() {
                Ok(30) => st.file.ended = true,
                Ok(15) => {
                    st.route_down = true;
                    st.current_route = Some(Route {
                        line,
                        tool: st.tool,
                        points: vec![Point { x: st.x, y: st.y }],
                        arcs: Vec::new(),
                    });
                }
                Ok(16) => {
                    st.route_down = false;
                    st.finish_route();
                }
                Ok(0 | 1) => {}
                Ok(71) => st.declare_units(Units::Millimetres, line),
                Ok(72) => st.declare_units(Units::Inches, line),
                _ => st.diag(
                    line,
                    Severity::Warning,
                    codes::UNKNOWN_M_CODE,
                    format!("unknown M{digits}"),
                ),
            },
            'X' | 'Y' | 'A' | 'I' | 'J' => {
                let normalised = if digits.contains(',') && !digits.contains('.') {
                    if !comma_told {
                        comma_told = true;
                        st.diag(
                            line,
                            Severity::Warning,
                            codes::DECIMAL_COMMA,
                            "a coordinate uses a comma as the decimal point; it was read as a point",
                        );
                    }
                    digits.replace(',', ".")
                } else {
                    digits.clone()
                };
                match coordinate(st, &normalised, line) {
                    Some(v) => {
                        let slot = match *letter {
                            'X' => &mut x,
                            'Y' => &mut y,
                            'A' => &mut radius,
                            'I' => &mut arc_i,
                            _ => &mut arc_j,
                        };
                        if slot.replace(v).is_some() {
                            st.diag(
                                line,
                                Severity::Error,
                                codes::WORD_REPEATED,
                                format!(
                                    "{letter} is written twice on a line: the last one is used"
                                ),
                            );
                        }
                    }
                    None => {
                        st.diag(
                            line,
                            Severity::Error,
                            coordinate_error_code(digits, int_digits, dec_digits),
                            format!("'{letter}{digits}' is not a valid coordinate"),
                        );
                        return;
                    }
                }
            }
            'F' | 'S' | 'N' | 'R' => {}
            other => st.diag(
                line,
                Severity::Warning,
                codes::UNKNOWN_WORD,
                format!("unknown word '{other}{digits}' was ignored"),
            ),
        }
    }
    let has = |letter: char, numbers: [u32; 2]| {
        words
            .iter()
            .any(|(l, d)| *l == letter && d.parse::<u32>().is_ok_and(|n| numbers.contains(&n)))
    };
    // `G92` / `G93` set the zero of the program: their coordinates are not a hole
    if has('G', [92, 93]) {
        if x.unwrap_or(0.0) == 0.0 && y.unwrap_or(0.0) == 0.0 {
            st.diag(
                line,
                Severity::Info,
                codes::COMMAND_IGNORED,
                "a zero set (G92, G93) at zero was ignored",
            );
        } else {
            st.diag(
                line,
                Severity::Error,
                codes::GEOMETRY_COMMAND_UNSUPPORTED,
                "a zero set (G92, G93) away from zero is not supported: the positions are not shifted",
            );
        }
        return;
    }
    // `M30X..Y..` and `M00X..Y..` say where the head stops at the end: not a hole
    if (x.is_some() || y.is_some()) && has('M', [0, 30]) {
        st.diag(
            line,
            Severity::Warning,
            codes::COMMAND_IGNORED,
            "the position on the end line (M00, M30) is where the head stops, not a hole",
        );
        return;
    }
    // `M02X<offset>` moves a repeated pattern: it is not read, and the offset must not become a hole of its own.
    let pattern_offset = (x.is_some() || y.is_some())
        && words
            .iter()
            .any(|(l, d)| *l == 'M' && d.parse::<u32>() == Ok(2));
    if pattern_offset {
        st.diag(
            line,
            Severity::Error,
            codes::GEOMETRY_COMMAND_UNSUPPORTED,
            "a pattern offset (M02 with a coordinate) is not supported: the holes it makes are missing",
        );
        return;
    }
    // `R<n>X<step>Y<step>` repeats the last hole n times, each one a step farther: X and Y are steps, not positions.
    if let Some((_, count)) = words.iter().find(|(l, _)| *l == 'R') {
        repeat_hole(st, count, x, y, line);
        return;
    }
    if x.is_none() && y.is_none() {
        return;
    }
    st.x = match x {
        Some(v) if st.incremental => st.x + v,
        Some(v) => v,
        None => st.x,
    };
    st.y = match y {
        Some(v) if st.incremental => st.y + v,
        Some(v) => v,
        None => st.y,
    };
    let at = Point { x: st.x, y: st.y };
    if st.routing && g.is_some() || st.routing && st.route_down {
        if st.route_down {
            let arc = st.mode != RouteMode::Linear;
            if arc && radius.is_none() && arc_i.is_none() && arc_j.is_none() {
                st.diag(
                    line,
                    Severity::Error,
                    codes::ARC_CENTRE_MISSING,
                    "an arc of a route (G02/G03) has neither a radius (A) nor offsets (I, J)",
                );
            }
            if !st.count_item(line) {
                return;
            }
            if let Some(route) = st.current_route.as_mut() {
                route.points.push(at);
                if arc {
                    route.arcs.push(RouteArc {
                        to_index: route.points.len() - 1,
                        clockwise: st.mode == RouteMode::Clockwise,
                        radius,
                        center_offset: if arc_i.is_some() || arc_j.is_some() {
                            Some((arc_i.unwrap_or(0.0), arc_j.unwrap_or(0.0)))
                        } else {
                            None
                        },
                    });
                }
            }
        } else if st.mode != RouteMode::Linear {
            st.diag(
                line,
                Severity::Error,
                codes::ARC_UNSUPPORTED,
                "an arc (G02/G03) outside a route (the tool is not down, M15) is not drawn",
            );
        }
        return;
    }
    if !st.count_item(line) {
        return;
    }
    match st.tool {
        Some(tool) if st.file.tools.contains_key(&tool) => {
            st.file.drills.push(Hit { line, tool, at })
        }
        Some(tool) => {
            st.diag(
                line,
                Severity::Error,
                codes::TOOL_UNDEFINED,
                format!("a hole uses the undefined tool T{tool}"),
            );
            // the hole is not kept: with no tool it would have a diameter of 0 and be drilled as a point
        }
        None => st.diag(
            line,
            Severity::Error,
            codes::TOOL_NOT_SELECTED,
            "a hole comes before any tool is selected",
        ),
    }
}

fn split_xy(st: &mut State, text: &str, line: usize) -> Option<Point> {
    let (mut x, mut y) = (None, None);
    let mut letter = ' ';
    let mut buffer = String::new();
    for c in text.chars().chain(std::iter::once('\0')) {
        if matches!(c, 'X' | 'Y' | '\0') {
            if letter != ' ' {
                match (coordinate(st, &std::mem::take(&mut buffer), line), letter) {
                    (Some(v), _) => {
                        let slot = if letter == 'X' { &mut x } else { &mut y };
                        if slot.replace(v).is_some() {
                            st.diag(
                                line,
                                Severity::Error,
                                codes::WORD_REPEATED,
                                format!(
                                    "{letter} is written twice in a slot: the last one is used"
                                ),
                            );
                        }
                    }
                    (None, _) => {
                        st.diag(
                            line,
                            Severity::Error,
                            codes::COORDINATE_INVALID,
                            "a slot has a coordinate that is not valid",
                        );
                        return None;
                    }
                }
            }
            letter = c;
        } else {
            buffer.push(c);
        }
    }
    let inc = st.incremental;
    Some(Point {
        x: match x {
            Some(v) if inc => st.x + v,
            Some(v) => v,
            None => st.x,
        },
        y: match y {
            Some(v) if inc => st.y + v,
            Some(v) => v,
            None => st.y,
        },
    })
}

fn repeat_hole(st: &mut State, count: &str, x: Option<f64>, y: Option<f64>, line: usize) {
    let Ok(n) = count.parse::<u32>() else {
        st.diag(
            line,
            Severity::Error,
            codes::GEOMETRY_COMMAND_UNSUPPORTED,
            "a repeat (R) has no count",
        );
        return;
    };
    if n > st.limits.max_repeat {
        st.diag(
            line,
            Severity::Error,
            codes::LIMIT_EXCEEDED,
            format!(
                "a repeat of {n} holes is more than the limit of {}: no hole was made",
                st.limits.max_repeat
            ),
        );
        return;
    }
    let Some(last) = st.file.drills.last().copied() else {
        st.diag(
            line,
            Severity::Error,
            codes::GEOMETRY_COMMAND_UNSUPPORTED,
            "a repeat (R) comes before any hole to repeat",
        );
        return;
    };
    let (dx, dy) = (x.unwrap_or(0.0), y.unwrap_or(0.0));
    for k in 1..=n {
        if !st.count_item(line) {
            return;
        }
        st.file.drills.push(Hit {
            line,
            tool: last.tool,
            at: Point {
                x: last.at.x + dx * f64::from(k),
                y: last.at.y + dy * f64::from(k),
            },
        });
    }
    if let Some(end) = st.file.drills.last() {
        st.x = end.at.x;
        st.y = end.at.y;
    }
}

fn slot(st: &mut State, before: &str, after: &str, line: usize) {
    let Some(from) = split_xy(st, before, line) else {
        return;
    };
    st.x = from.x;
    st.y = from.y;
    if !after.contains(['X', 'Y']) {
        st.diag(
            line,
            Severity::Error,
            codes::COORDINATE_MISSING,
            "a slot (G85) has no end: the line may be cut",
        );
        return;
    }
    let Some(to) = split_xy(st, after, line) else {
        return;
    };
    st.x = to.x;
    st.y = to.y;
    match st.tool {
        Some(tool) => {
            if !st.file.tools.contains_key(&tool) {
                st.diag(
                    line,
                    Severity::Error,
                    codes::TOOL_UNDEFINED,
                    format!("a slot uses the undefined tool T{tool}"),
                );
            }
            if !st.count_item(line) {
                return;
            }
            st.file.slots.push(Slot {
                line,
                tool,
                from,
                to,
            });
        }
        None => st.diag(
            line,
            Severity::Error,
            codes::TOOL_NOT_SELECTED,
            "a slot comes before any tool is selected",
        ),
    }
}

fn current_format(st: &State) -> (u32, u32) {
    st.format.unwrap_or(match st.file.units {
        Some(Units::Inches) => (2, 4),
        _ => (3, 3),
    })
}

fn coordinate(st: &mut State, digits: &str, line: usize) -> Option<f64> {
    let (integer, decimal) = current_format(st);
    let value = decode_coordinate(
        digits,
        integer,
        decimal,
        st.file.zeros.unwrap_or(ZeroSuppression::Leading),
    )?;
    note_assumptions(st, digits, value, line);
    Some(value)
}

/// Tells, once each, that a position rests on a supposition. A coordinate with a point, and a zero, mean the same
/// under any format; the zero suppression changes nothing in a coordinate that has all the digits of its format.
fn note_assumptions(st: &mut State, digits: &str, value: f64, line: usize) {
    if value == 0.0 || digits.contains('.') {
        return;
    }
    let (integer, decimal) = current_format(st);
    if st.format.is_none() && !st.format_told {
        st.format_told = true;
        st.keep(
            line,
            Severity::Warning,
            codes::FORMAT_ASSUMED,
            format!(
                "no coordinate format is declared: '{digits}' is read with {integer} integer and {decimal} decimal digits"
            ),
        );
    }
    let written = digits.trim_start_matches(['+', '-']);
    let width = integer.saturating_add(decimal) as usize;
    if st.file.zeros.is_none() && !st.zeros_told && written.len() < width {
        st.zeros_told = true;
        let hint = if written.starts_with('0') {
            " (it starts with a zero, so the file may leave out the trailing ones instead)"
        } else {
            ""
        };
        st.keep(
            line,
            Severity::Warning,
            codes::ZEROS_ASSUMED,
            format!(
                "no zero suppression is declared: '{digits}' is read with the leading zeros left out{hint}"
            ),
        );
    }
}
