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
//! Not done: incremental coordinates, arcs in routes (`G02`/`G03`) and any drawing of geometry.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::string_slice)]

use std::collections::BTreeMap;

pub use eh3d_parser_common::{
    codes, decode_coordinate, Diagnostic, Severity, Units, ZeroSuppression,
};
use eh3d_parser_common::{coordinate_error_code, decode_input, parse_f64, resolve_units};
pub use eh3d_parser_common::{read_file_limited, InputError, InputLimits, ReadError};

/// A file with no units whose holes span less than this many units may be in inches (no board is 5 mm wide).
const SUSPECT_SPAN: f64 = 5.0;

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

/// A routed path: tool down at the first point, straight segments to the others.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// The line where the tool went down.
    pub line: usize,
    /// The tool.
    pub tool: Option<u32>,
    /// The points of the path.
    pub points: Vec<Point>,
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
}

struct State {
    file: ExcellonFile,
    format: Option<(u32, u32)>,
    header: bool,
    tool: Option<u32>,
    x: f64,
    y: f64,
    routing: bool,
    route_down: bool,
    current_route: Option<Route>,
}

impl State {
    fn diag(
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

/// Reads an Excellon file. Never fails: problems are in [`ExcellonFile::diagnostics`].
pub fn parse(text: &str) -> ExcellonFile {
    let mut st = State {
        file: ExcellonFile {
            units: None,
            zeros: None,
            tools: BTreeMap::new(),
            drills: Vec::new(),
            slots: Vec::new(),
            routes: Vec::new(),
            ended: false,
            diagnostics: Vec::new(),
        },
        format: None,
        header: false,
        tool: None,
        x: 0.0,
        y: 0.0,
        routing: false,
        route_down: false,
        current_route: None,
    };
    let mut started = false;
    for (index, raw) in text.lines().enumerate() {
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

fn comment_line(st: &mut State, comment: &str, line: usize) {
    // KiCad and others state the integer format in a comment: FILE_FORMAT=4:3
    let compact: String = comment.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(rest) = compact.to_ascii_uppercase().strip_prefix("FILE_FORMAT=") {
        if let Some((i, d)) = rest.split_once(':') {
            match (i.parse::<u32>(), d.parse::<u32>()) {
                (Ok(i), Ok(d)) if (1..=8).contains(&i) && d <= 8 => st.format = Some((i, d)),
                _ => st.diag(
                    line,
                    Severity::Warning,
                    codes::FORMAT_COMMENT_INVALID,
                    "FILE_FORMAT comment is not understood",
                ),
            }
        }
    }
}

fn header_line(st: &mut State, upper: &str, line: usize) {
    let (word, rest) = upper.split_once(',').unwrap_or((upper, ""));
    match word {
        // the old way of saying the units: M71 millimetres, M72 inches
        "M71" | "M72" => {
            st.file.units = Some(if word == "M71" {
                Units::Millimetres
            } else {
                Units::Inches
            });
        }
        "METRIC" | "INCH" => {
            st.file.units = Some(if word == "METRIC" {
                Units::Millimetres
            } else {
                Units::Inches
            });
            for part in rest.split(',').filter(|p| !p.is_empty()) {
                match part {
                    "TZ" => st.file.zeros = Some(ZeroSuppression::Leading),
                    "LZ" => st.file.zeros = Some(ZeroSuppression::Trailing),
                    other if other.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                        // 000.000 or 00.0000: the number of digits before and after the point
                        if let Some((i, d)) = other.split_once('.') {
                            st.format = Some((i.len() as u32, d.len() as u32));
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
        "FMAT" | "VER" | "DETECT" | "ATC" | "ICI" if word == "ICI" => {
            if rest != "OFF" {
                st.diag(
                    line,
                    Severity::Error,
                    codes::INCREMENTAL_UNSUPPORTED,
                    "incremental coordinates (ICI) are not supported",
                );
            }
        }
        "FMAT" | "VER" | "DETECT" | "ATC" => st.diag(
            line,
            Severity::Info,
            codes::COMMAND_IGNORED,
            format!("{word} was ignored"),
        ),
        _ if upper.starts_with('T') => tool_definition(st, upper, line),
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

fn tool_definition(st: &mut State, upper: &str, line: usize) {
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
    let (mut x, mut y) = (None, None);
    let mut g: Option<u32> = None;
    let (int_digits, dec_digits) = current_format(st);
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
                }
                Ok(2 | 3) => st.diag(
                    line,
                    Severity::Error,
                    codes::ARC_UNSUPPORTED,
                    "arcs in routes (G02/G03) are not supported",
                ),
                Ok(5) => {
                    st.routing = false;
                    st.finish_route();
                }
                Ok(90) => {}
                Ok(91) => st.diag(
                    line,
                    Severity::Error,
                    codes::INCREMENTAL_UNSUPPORTED,
                    "G91 (incremental coordinates) is not supported",
                ),
                Ok(93) => st.diag(
                    line,
                    Severity::Info,
                    codes::COMMAND_IGNORED,
                    "G93 (zero set) was ignored",
                ),
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
                    });
                }
                Ok(16) => {
                    st.route_down = false;
                    st.finish_route();
                }
                Ok(0 | 1) => {}
                Ok(71) => st.file.units = Some(Units::Millimetres),
                Ok(72) => st.file.units = Some(Units::Inches),
                _ => st.diag(
                    line,
                    Severity::Warning,
                    codes::UNKNOWN_M_CODE,
                    format!("unknown M{digits}"),
                ),
            },
            'X' | 'Y' => match coordinate(st, digits) {
                Some(v) if *letter == 'X' => x = Some(v),
                Some(v) => y = Some(v),
                None => {
                    st.diag(
                        line,
                        Severity::Error,
                        coordinate_error_code(digits, int_digits, dec_digits),
                        format!("'{letter}{digits}' is not a valid coordinate"),
                    );
                    return;
                }
            },
            'F' | 'S' | 'N' => {}
            other => st.diag(
                line,
                Severity::Warning,
                codes::UNKNOWN_WORD,
                format!("unknown word '{other}{digits}' was ignored"),
            ),
        }
    }
    if x.is_none() && y.is_none() {
        return;
    }
    st.x = x.unwrap_or(st.x);
    st.y = y.unwrap_or(st.y);
    let at = Point { x: st.x, y: st.y };
    if st.routing && g.is_some() || st.routing && st.route_down {
        if st.route_down {
            if let Some(route) = st.current_route.as_mut() {
                route.points.push(at);
            }
        }
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
                match (coordinate(st, &std::mem::take(&mut buffer)), letter) {
                    (Some(v), 'X') => x = Some(v),
                    (Some(v), _) => y = Some(v),
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
    Some(Point {
        x: x.unwrap_or(st.x),
        y: y.unwrap_or(st.y),
    })
}

fn slot(st: &mut State, before: &str, after: &str, line: usize) {
    let Some(from) = split_xy(st, before, line) else {
        return;
    };
    st.x = from.x;
    st.y = from.y;
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

fn coordinate(st: &mut State, digits: &str) -> Option<f64> {
    let (integer, decimal) = current_format(st);
    decode_coordinate(
        digits,
        integer,
        decimal,
        st.file.zeros.unwrap_or(ZeroSuppression::Leading),
    )
}
