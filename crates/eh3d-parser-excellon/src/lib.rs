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

use std::collections::BTreeMap;

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something worth knowing.
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

/// The units of the coordinates and tool diameters.
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

/// Which zeros an integer coordinate leaves out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroSuppression {
    /// Leading zeros are left out (`TZ`).
    Leading,
    /// Trailing zeros are left out (`LZ`).
    Trailing,
}

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
    fn diag(&mut self, line: usize, severity: Severity, message: impl Into<String>) {
        self.file.diagnostics.push(Diagnostic {
            line,
            severity,
            message: message.into(),
        });
    }

    fn finish_route(&mut self) {
        if let Some(route) = self.current_route.take() {
            self.file.routes.push(route);
        }
    }
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
            st.diag(line, Severity::Warning, "text after M30 was ignored");
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
    if !started {
        st.diag(1, Severity::Warning, "the file has no M48 header");
    }
    if !st.file.ended {
        let last = text.lines().count().max(1);
        st.diag(last, Severity::Warning, "the file does not end with M30");
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
                    "FILE_FORMAT comment is not understood",
                ),
            }
        }
    }
}

fn parse_f64(text: &str) -> Option<f64> {
    let value: f64 = text.parse().ok()?;
    value.is_finite().then_some(value)
}

fn header_line(st: &mut State, upper: &str, line: usize) {
    let (word, rest) = upper.split_once(',').unwrap_or((upper, ""));
    match word {
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
                    "incremental coordinates (ICI) are not supported",
                );
            }
        }
        "FMAT" | "VER" | "DETECT" | "ATC" => {
            st.diag(line, Severity::Info, format!("{word} was ignored"))
        }
        _ if upper.starts_with('T') => tool_definition(st, upper, line),
        _ => st.diag(
            line,
            Severity::Warning,
            format!(
                "unknown header line '{}'",
                upper.chars().take(20).collect::<String>()
            ),
        ),
    }
}

fn tool_definition(st: &mut State, upper: &str, line: usize) {
    let digits: String = upper[1..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let Ok(number) = digits.parse::<u32>() else {
        st.diag(
            line,
            Severity::Error,
            "a tool definition has no tool number",
        );
        return;
    };
    let mut rest = &upper[1 + digits.len()..];
    let mut tool = Tool {
        number,
        diameter: 0.0,
        feed: None,
        speed: None,
    };
    let mut have_diameter = false;
    while let Some(letter) = rest.chars().next() {
        let number_text: String = rest[1..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
            .collect();
        let advance = 1 + number_text.len();
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
                    "a tool has a diameter that is not a positive number",
                );
                return;
            }
            (other, _) => {
                st.diag(
                    line,
                    Severity::Warning,
                    format!("unknown tool parameter '{other}' was ignored"),
                );
            }
        }
        rest = &rest[advance..];
    }
    if !have_diameter {
        st.diag(
            line,
            Severity::Error,
            format!("tool T{number} is defined without a diameter"),
        );
        return;
    }
    if st.file.tools.insert(number, tool).is_some() {
        st.diag(
            line,
            Severity::Warning,
            format!("tool T{number} is defined twice; the last definition wins"),
        );
    }
}

fn body_line(st: &mut State, upper: &str, line: usize) {
    if let Some(index) = upper.find("G85") {
        slot(st, &upper[..index], &upper[index + 3..], line);
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
            format!(
                "'{}' was ignored",
                upper.chars().take(20).collect::<String>()
            ),
        );
        return;
    }
    let (mut x, mut y) = (None, None);
    let mut g: Option<u32> = None;
    for (letter, digits) in &words {
        match *letter {
            'T' => match digits.parse::<u32>() {
                Ok(0) => st.tool = None,
                Ok(n) => {
                    if !st.file.tools.contains_key(&n) {
                        st.diag(
                            line,
                            Severity::Error,
                            format!("T{n} is selected but never defined"),
                        );
                    }
                    st.tool = Some(n);
                }
                Err(_) => st.diag(line, Severity::Error, "a tool selection has no number"),
            },
            'G' => match digits.parse::<u32>() {
                Ok(n @ (0 | 1)) => {
                    g = Some(n);
                    st.routing = true;
                }
                Ok(2 | 3) => st.diag(
                    line,
                    Severity::Error,
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
                    "G91 (incremental coordinates) is not supported",
                ),
                Ok(93) => st.diag(line, Severity::Info, "G93 (zero set) was ignored"),
                _ => st.diag(line, Severity::Warning, format!("unknown G{digits}")),
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
                _ => st.diag(line, Severity::Warning, format!("unknown M{digits}")),
            },
            'X' | 'Y' => match coordinate(st, digits) {
                Some(v) if *letter == 'X' => x = Some(v),
                Some(v) => y = Some(v),
                None => {
                    st.diag(
                        line,
                        Severity::Error,
                        format!("'{letter}{digits}' is not a valid coordinate"),
                    );
                    return;
                }
            },
            'F' | 'S' | 'N' => {}
            other => st.diag(
                line,
                Severity::Warning,
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
                format!("a hole uses the undefined tool T{tool}"),
            );
            st.file.drills.push(Hit { line, tool, at });
        }
        None => st.diag(
            line,
            Severity::Error,
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
            "a slot comes before any tool is selected",
        ),
    }
}

fn coordinate(st: &mut State, digits: &str) -> Option<f64> {
    let units = st.file.units;
    let (integer, decimal) = st.format.unwrap_or(match units {
        Some(Units::Inches) => (2, 4),
        _ => (3, 3),
    });
    decode_coordinate(
        digits,
        integer,
        decimal,
        st.file.zeros.unwrap_or(ZeroSuppression::Leading),
    )
}

/// Converts the digits of a coordinate to a number.
///
/// A value with a decimal point is read as it is. Otherwise the digits are an integer: with
/// leading zeros suppressed it is divided by 10^decimals; with trailing zeros suppressed the digits
/// are first padded on the right to the full width of the format.
pub fn decode_coordinate(
    text: &str,
    integer_digits: u32,
    decimal_digits: u32,
    zeros: ZeroSuppression,
) -> Option<f64> {
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
        let width = (integer_digits + decimal_digits) as usize;
        if digits.len() > width {
            return None;
        }
        let padded = match zeros {
            ZeroSuppression::Leading => digits.to_owned(),
            ZeroSuppression::Trailing => format!("{digits:0<width$}"),
        };
        let value: f64 = padded.parse().ok()?;
        value / 10f64.powi(decimal_digits as i32)
    };
    Some(if negative { -magnitude } else { magnitude })
}
