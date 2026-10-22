//! # eh3d-parser-gcode
//!
//! A tolerant reader of G-code with a **modal interpreter**: [`parse`] reads the text of a program and
//! [`interpret`] walks it the way a machine controller does, keeping the modal state (units, distance mode,
//! feed rate, spindle, tool, work coordinate system) and turning every motion into a [`Segment`] with absolute
//! coordinates in **millimetres**, whatever the units of the file.
//!
//! Three dialects are known ([`Dialect`]): GRBL, Marlin and PlanetCNC. They differ in comments, checksums and in a few
//! M-codes; what a dialect does not know is reported as a warning, never silently dropped.
//!
//! Nothing here panics on any input. A line that cannot be understood becomes a [`Diagnostic`] with its line number and the
//! reading goes on with the next one.
//!
//! Not done: cutter-radius compensation (G41/G42), canned cycles (G81..G89), tool length offsets and
//! the Z-probing cycles; they are reported as unsupported.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// How serious a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Something worth knowing.
    Info,
    /// The program is probably still read correctly, but it is not clean.
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

/// The flavour of G-code of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// GRBL: `;` and `( )` comments, laser mode with M3/M4, `$` system commands.
    Grbl,
    /// Marlin: `;` comments, an optional line number and checksum (`N12 G1 X1*45`), M104/M109 and friends.
    Marlin,
    /// PlanetCNC: `;` and `( )` comments and its own M-codes for outputs.
    PlanetCnc,
}

/// One word of a block: a letter and its number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Word {
    /// The letter, in upper case.
    pub letter: char,
    /// The value.
    pub value: f64,
}

/// A line of the program, split into words.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The line in the file.
    pub line: usize,
    /// The words in the order they were written.
    pub words: Vec<Word>,
    /// The text of the comment of the line, if it had one.
    pub comment: Option<String>,
}

impl Block {
    /// The value of the last word with this letter.
    pub fn get(&self, letter: char) -> Option<f64> {
        self.words
            .iter()
            .rev()
            .find(|w| w.letter == letter)
            .map(|w| w.value)
    }
}

/// A program as it was read.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// The dialect it was read as.
    pub dialect: Dialect,
    /// The blocks that carry something to execute (blank and comment-only lines are left out).
    pub blocks: Vec<Block>,
    /// What the reader noticed.
    pub diagnostics: Vec<Diagnostic>,
}

impl Program {
    /// True when no diagnostic is of severity `Error`.
    pub fn is_clean(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|d| d.severity < Severity::Error)
    }
}

/// The longest line the reader looks at, in characters; a longer one is cut and reported.
pub const MAX_LINE_CHARS: usize = 512;

/// Reads a program. Never fails: problems are in [`Program::diagnostics`].
pub fn parse(text: &str, dialect: Dialect) -> Program {
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let raw = raw.trim_start_matches('\u{feff}');
        let mut raw = raw;
        if raw.chars().count() > MAX_LINE_CHARS {
            let cut = raw
                .char_indices()
                .nth(MAX_LINE_CHARS)
                .map(|(i, _)| i)
                .unwrap_or(raw.len());
            raw = &raw[..cut];
            diagnostics.push(Diagnostic {
                line,
                severity: Severity::Warning,
                message: format!("the line is longer than {MAX_LINE_CHARS} characters and was cut"),
            });
        }
        if let Some(block) = parse_line(raw, line, dialect, &mut diagnostics) {
            blocks.push(block);
        }
    }
    Program {
        dialect,
        blocks,
        diagnostics,
    }
}

fn parse_line(
    raw: &str,
    line: usize,
    dialect: Dialect,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Block> {
    let mut body = raw.trim();
    if body.starts_with('/') {
        diagnostics.push(Diagnostic {
            line,
            severity: Severity::Info,
            message: "a block-delete line was read as if the switch were off".to_string(),
        });
        body = body[1..].trim_start();
    }
    if body.starts_with('$') && dialect == Dialect::Grbl {
        diagnostics.push(Diagnostic {
            line,
            severity: Severity::Info,
            message: "a GRBL system command ($) was ignored".to_string(),
        });
        return None;
    }
    // Marlin: a checksum after '*' is verified and cut
    if dialect == Dialect::Marlin {
        if let Some((content, sum)) = body.split_once('*') {
            let computed = content.bytes().fold(0u8, |a, b| a ^ b);
            match sum
                .trim()
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .parse::<u32>()
            {
                Ok(expected) if expected == u32::from(computed) => {}
                Ok(_) => diagnostics.push(Diagnostic {
                    line,
                    severity: Severity::Error,
                    message: "the checksum of the line does not match".to_string(),
                }),
                Err(_) => diagnostics.push(Diagnostic {
                    line,
                    severity: Severity::Warning,
                    message: "the checksum of the line is not a number".to_string(),
                }),
            }
            body = content;
        }
    }
    let mut words = Vec::new();
    let mut comment: Option<String> = None;
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == ';' {
            let text = chars[i + 1..].iter().collect::<String>().trim().to_string();
            comment = Some(match comment {
                Some(old) => format!("{old} {text}"),
                None => text,
            });
            break;
        } else if c == '(' {
            let start = i + 1;
            while i < chars.len() && chars[i] != ')' {
                i += 1;
            }
            if i >= chars.len() {
                diagnostics.push(Diagnostic {
                    line,
                    severity: Severity::Warning,
                    message: "a comment in parentheses is not closed".to_string(),
                });
            }
            let text: String = chars[start..i.min(chars.len())].iter().collect();
            comment = Some(match comment {
                Some(old) => format!("{old} {text}"),
                None => text,
            });
            i += 1;
        } else if c.is_ascii_alphabetic() {
            let letter = c.to_ascii_uppercase();
            i += 1;
            let start = i;
            while i < chars.len() && matches!(chars[i], '0'..='9' | '.' | '-' | '+') {
                i += 1;
            }
            let number: String = chars[start..i].iter().collect();
            match number.parse::<f64>() {
                Ok(value) if value.is_finite() => words.push(Word { letter, value }),
                _ => diagnostics.push(Diagnostic {
                    line,
                    severity: Severity::Error,
                    message: format!("the word '{letter}{number}' has no valid number"),
                }),
            }
        } else {
            diagnostics.push(Diagnostic {
                line,
                severity: Severity::Warning,
                message: format!("the character '{c}' was ignored"),
            });
            i += 1;
        }
    }
    if words.is_empty() {
        return None;
    }
    Some(Block {
        line,
        words,
        comment,
    })
}

/// A point in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point3 {
    /// X.
    pub x: f64,
    /// Y.
    pub y: f64,
    /// Z.
    pub z: f64,
}

/// What kind of motion a [`Segment`] is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    /// Rapid positioning (G0).
    Rapid,
    /// Straight cut at the feed rate (G1).
    Linear,
    /// Circular cut, clockwise (G2); `center` is the centre of the arc in millimetres.
    ArcCw {
        /// The centre of the arc.
        center: (f64, f64),
    },
    /// Circular cut, counter-clockwise (G3).
    ArcCcw {
        /// The centre of the arc.
        center: (f64, f64),
    },
}

/// One motion with absolute coordinates in millimetres and the state it was made in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// The line in the file.
    pub line: usize,
    /// The kind of motion.
    pub motion: Motion,
    /// Where it starts.
    pub from: Point3,
    /// Where it ends.
    pub to: Point3,
    /// The feed rate in millimetres per minute (0 when none was set).
    pub feed: f64,
    /// The spindle speed (S word), 0 when none was set.
    pub spindle_speed: f64,
    /// Whether the spindle (or laser) was on.
    pub spindle_on: bool,
    /// The tool selected.
    pub tool: Option<u32>,
}

/// The result of walking a program.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Trace {
    /// The motions in order.
    pub segments: Vec<Segment>,
    /// The smallest corner of everything that moved, in millimetres.
    pub min: Point3,
    /// The largest corner of everything that moved, in millimetres.
    pub max: Point3,
    /// The length of the cutting motions (everything but rapids) in millimetres; arcs by their arc length.
    pub cutting_length_mm: f64,
    /// The length of the rapids in millimetres.
    pub rapid_length_mm: f64,
    /// Whether the program ended with M2 or M30.
    pub ended: bool,
    /// What the interpreter noticed.
    pub diagnostics: Vec<Diagnostic>,
}

const MM_PER_INCH: f64 = 25.4;

struct State {
    inches: bool,
    absolute: bool,
    absolute_arc_centre: bool,
    position: Point3,
    feed: f64,
    speed: f64,
    spindle_on: bool,
    tool: Option<u32>,
    motion: Option<u32>,
    seen_move: bool,
}

/// Walks a program, keeping the modal state, and returns its motions.
pub fn interpret(program: &Program) -> Trace {
    let mut state = State {
        inches: false,
        absolute: true,
        absolute_arc_centre: false,
        position: Point3::default(),
        feed: 0.0,
        speed: 0.0,
        spindle_on: false,
        tool: None,
        motion: None,
        seen_move: false,
    };
    let mut trace = Trace {
        diagnostics: program.diagnostics.clone(),
        ..Trace::default()
    };
    for block in &program.blocks {
        if trace.ended {
            trace.diagnostics.push(Diagnostic {
                line: block.line,
                severity: Severity::Warning,
                message: "the program went on after M2/M30".to_string(),
            });
            break;
        }
        run_block(block, &mut state, &mut trace, program.dialect);
    }
    trace
}

fn diag(trace: &mut Trace, line: usize, severity: Severity, message: impl Into<String>) {
    trace.diagnostics.push(Diagnostic {
        line,
        severity,
        message: message.into(),
    });
}

fn run_block(block: &Block, st: &mut State, trace: &mut Trace, dialect: Dialect) {
    let scale = |st: &State, v: f64| if st.inches { v * MM_PER_INCH } else { v };
    let mut skip_motion = false; // a block with a feature that is not supported does not move anything
                                 // 1. the modal words, in the order of the block
    for word in &block.words {
        let v = word.value;
        match word.letter {
            'G' => {
                let code = (v * 10.0).round() as i64; // G91.1 -> 911
                match code {
                    0 | 10 | 20 | 30 => {
                        st.motion = Some((code / 10) as u32);
                    }
                    200 => st.inches = true,
                    210 => st.inches = false,
                    900 => st.absolute = true,
                    910 => st.absolute = false,
                    901 => st.absolute_arc_centre = true,
                    911 => st.absolute_arc_centre = false,
                    170 | 180 | 190 => {
                        if code != 170 {
                            diag(
                                trace,
                                block.line,
                                Severity::Warning,
                                "only the XY plane (G17) is supported for arcs",
                            );
                        }
                    }
                    40 | 400 | 280 | 300 | 540 | 550 | 560 | 570 | 580 | 590 | 530 | 920 | 921
                    | 922 | 930 | 940 | 500 | 610 | 640 | 490 | 430 | 440 => {
                        if matches!(code, 280 | 300 | 920 | 530) {
                            diag(
                                trace,
                                block.line,
                                Severity::Info,
                                format!(
                                    "G{} was read but its effect on the position is not tracked",
                                    v
                                ),
                            );
                        }
                    }
                    410 | 420 => {
                        skip_motion = true;
                        diag(
                            trace,
                            block.line,
                            Severity::Error,
                            "cutter radius compensation (G41/G42) is not supported",
                        );
                    }
                    c if (810..=890).contains(&c) => {
                        skip_motion = true;
                        diag(
                            trace,
                            block.line,
                            Severity::Error,
                            format!("canned cycles (G{v}) are not supported"),
                        );
                    }
                    _ => diag(
                        trace,
                        block.line,
                        Severity::Warning,
                        format!("G{v} is not known and was ignored"),
                    ),
                }
            }
            'M' => match v as i64 {
                0 | 1 => {}
                2 | 30 => trace.ended = true,
                3 | 4 => st.spindle_on = true,
                5 => st.spindle_on = false,
                6 => {}
                7..=9 => {}
                104 | 109 | 140 | 190 | 106 | 107 if dialect == Dialect::Marlin => {}
                _ => diag(
                    trace,
                    block.line,
                    Severity::Warning,
                    format!("M{v} is not known and was ignored"),
                ),
            },
            'F' => {
                if v < 0.0 {
                    diag(
                        trace,
                        block.line,
                        Severity::Error,
                        "a negative feed rate was ignored",
                    );
                } else {
                    st.feed = scale(st, v);
                }
            }
            'S' => {
                if v < 0.0 {
                    diag(
                        trace,
                        block.line,
                        Severity::Error,
                        "a negative spindle speed was ignored",
                    );
                } else {
                    st.speed = v;
                }
            }
            'T' => {
                if v >= 0.0 && v.fract() == 0.0 && v <= f64::from(u32::MAX) {
                    st.tool = Some(v as u32);
                } else {
                    diag(
                        trace,
                        block.line,
                        Severity::Error,
                        "a tool number that is not a whole number was ignored",
                    );
                }
            }
            'P' | 'L' | 'N' | 'H' | 'D' | 'E' | 'R' | 'X' | 'Y' | 'Z' | 'I' | 'J' | 'K' | 'A'
            | 'B' | 'C' | 'U' | 'V' | 'W' | 'Q' => {}
            other => diag(
                trace,
                block.line,
                Severity::Warning,
                format!("the word '{other}' is not known and was ignored"),
            ),
        }
    }
    // 2. the motion of the block
    let has_axis = block
        .words
        .iter()
        .any(|w| matches!(w.letter, 'X' | 'Y' | 'Z'));
    if !has_axis || skip_motion {
        return;
    }
    let code = match st.motion {
        Some(code) => code,
        None => {
            if !st.seen_move {
                diag(
                    trace,
                    block.line,
                    Severity::Warning,
                    "an axis word comes before any G0/G1/G2/G3: it was taken as a rapid (G0)",
                );
            }
            st.motion = Some(0);
            0
        }
    };
    run_motion(block, st, trace, code, scale);
}

fn run_motion(
    block: &Block,
    st: &mut State,
    trace: &mut Trace,
    code: u32,
    scale: impl Fn(&State, f64) -> f64,
) {
    st.seen_move = true;
    let from = st.position;
    let target = |letter: char, current: f64, st: &State| -> f64 {
        match block.get(letter) {
            Some(v) => {
                let mm = scale(st, v);
                if st.absolute {
                    mm
                } else {
                    current + mm
                }
            }
            None => current,
        }
    };
    let to = Point3 {
        x: target('X', from.x, st),
        y: target('Y', from.y, st),
        z: target('Z', from.z, st),
    };
    let motion = match code {
        0 => Motion::Rapid,
        1 => Motion::Linear,
        2 | 3 => {
            let (i, j, r) = (block.get('I'), block.get('J'), block.get('R'));
            let centre = if i.is_some() || j.is_some() {
                let i_mm = scale(st, i.unwrap_or(0.0));
                let j_mm = scale(st, j.unwrap_or(0.0));
                Some(if st.absolute_arc_centre {
                    (i_mm, j_mm)
                } else {
                    (from.x + i_mm, from.y + j_mm)
                })
            } else if let Some(r) = r {
                match centre_from_radius(from, to, scale(st, r), code == 2) {
                    Some(c) => Some(c),
                    None => {
                        diag(trace, block.line, Severity::Error, "an arc with a radius (R) that does not reach from its start to its end was drawn as a straight line");
                        None
                    }
                }
            } else {
                diag(
                    trace,
                    block.line,
                    Severity::Error,
                    "an arc has no centre (I/J) and no radius (R): it was drawn as a straight line",
                );
                None
            };
            match centre {
                Some(center) if code == 2 => Motion::ArcCw { center },
                Some(center) => Motion::ArcCcw { center },
                None => Motion::Linear,
            }
        }
        _ => Motion::Linear,
    };
    if !matches!(motion, Motion::Rapid) && st.feed == 0.0 {
        diag(
            trace,
            block.line,
            Severity::Warning,
            "a cutting move was made with no feed rate set",
        );
    }
    st.position = to;
    push(block, st, trace, motion, from, to);
}

fn push(block: &Block, st: &State, trace: &mut Trace, motion: Motion, from: Point3, to: Point3) {
    let segment = Segment {
        line: block.line,
        motion,
        from,
        to,
        feed: st.feed,
        spindle_speed: st.speed,
        spindle_on: st.spindle_on,
        tool: st.tool,
    };
    let length = match motion {
        Motion::ArcCw { center } | Motion::ArcCcw { center } => arc_length(&segment, center),
        _ => ((to.x - from.x).powi(2) + (to.y - from.y).powi(2) + (to.z - from.z).powi(2)).sqrt(),
    };
    if matches!(motion, Motion::Rapid) {
        trace.rapid_length_mm += length;
    } else {
        trace.cutting_length_mm += length;
    }
    if trace.segments.is_empty() {
        trace.min = from;
        trace.max = from;
    }
    for p in [from, to] {
        trace.min = Point3 {
            x: trace.min.x.min(p.x),
            y: trace.min.y.min(p.y),
            z: trace.min.z.min(p.z),
        };
        trace.max = Point3 {
            x: trace.max.x.max(p.x),
            y: trace.max.y.max(p.y),
            z: trace.max.z.max(p.z),
        };
    }
    trace.segments.push(segment);
}

/// The centre of the arc that goes from `from` to `to` with radius `r` (positive: the smaller arc, negative: the larger one), or None when the points are
/// further apart than the diameter or coincide.
fn centre_from_radius(from: Point3, to: Point3, r: f64, clockwise: bool) -> Option<(f64, f64)> {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    let d2 = dx * dx + dy * dy;
    if d2 == 0.0 || r == 0.0 {
        return None;
    }
    let radius = r.abs();
    let h2 = radius * radius - d2 / 4.0;
    if h2 < -1e-9 {
        return None;
    }
    let h = h2.max(0.0).sqrt();
    let d = d2.sqrt();
    let (mx, my) = ((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
    // seen from the start, the centre of a clockwise minor arc is to the right of the chord and that of a counter-clockwise one to the left
    let (nx, ny) = if clockwise {
        (dy / d, -dx / d)
    } else {
        (-dy / d, dx / d)
    };
    let side = if r > 0.0 { 1.0 } else { -1.0 };
    Some((mx + side * h * nx, my + side * h * ny))
}

/// The length of the arc of a segment around `center`; a full circle when the end is the start.
fn arc_length(segment: &Segment, center: (f64, f64)) -> f64 {
    let (from, to) = (segment.from, segment.to);
    let radius = ((from.x - center.0).powi(2) + (from.y - center.1).powi(2)).sqrt();
    let a0 = (from.y - center.1).atan2(from.x - center.0);
    let a1 = (to.y - center.1).atan2(to.x - center.0);
    let tau = std::f64::consts::TAU;
    let mut sweep = match segment.motion {
        Motion::ArcCw { .. } => a0 - a1,
        _ => a1 - a0,
    };
    while sweep <= 0.0 {
        sweep += tau;
    }
    while sweep > tau {
        sweep -= tau;
    }
    if (from.x - to.x).abs() < 1e-9 && (from.y - to.y).abs() < 1e-9 {
        sweep = tau;
    }
    let flat = radius * sweep;
    (flat * flat + (to.z - from.z).powi(2)).sqrt()
}
