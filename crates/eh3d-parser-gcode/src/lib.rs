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
//! [`interpret_coordinates`] also follows `G92`, `G28`, `G30`, `G53` and `G54`..`G59` with the positions and limits of a
//! [`CoordinateOptions`]; [`interpret`] leaves them untracked and says so.
//!
//! Not done: cutter-radius compensation (G41/G42), canned cycles (G81..G89), tool length offsets and
//! the Z-probing cycles; they are reported as unsupported.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::string_slice)]

pub mod emit;
pub mod modal;

pub use modal::ModalState;

pub use eh3d_parser_common::{
    codes, read_file_limited, Diagnostic, InputError, InputLimits, ReadError, Severity,
};
use eh3d_parser_common::{decode_input, parse_number, NumberError};

/// The flavour of G-code of a program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// GRBL: `;` and `( )` comments, laser mode with M3/M4, `$` system commands. Spaces and comments count for nothing, so `X1 000` is `X1000`, as the controller reads it.
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

/// Reads a program from bytes: UTF-8 (with or without a byte order mark) or, with a warning
/// (`input-not-utf8`), Latin-1. An input above the default limit of 256 MiB is refused.
pub fn parse_bytes(bytes: &[u8], dialect: Dialect) -> Result<Program, InputError> {
    parse_bytes_with(bytes, dialect, &InputLimits::default())
}

/// Like [`parse_bytes`] with a limit of your own.
pub fn parse_bytes_with(
    bytes: &[u8],
    dialect: Dialect,
    limits: &InputLimits,
) -> Result<Program, InputError> {
    let input = decode_input(bytes, limits)?;
    let mut program = parse(&input.text, dialect);
    program.diagnostics.splice(0..0, input.diagnostics);
    Ok(program)
}

/// Reads a program. Never fails: problems are in [`Program::diagnostics`].
pub fn parse(text: &str, dialect: Dialect) -> Program {
    parse_with_limits(text, dialect, &ParseLimits::default())
}

/// The most blocks the reader keeps by default.
pub const MAX_BLOCKS: usize = 5_000_000;

/// The most diagnostics the reader keeps by default.
pub const MAX_DIAGNOSTICS: usize = 100_000;

/// How much a program may grow in memory while it is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseLimits {
    /// The most blocks kept; the rest of the program is not read (program-too-long).
    pub max_blocks: usize,
    /// The most diagnostics kept; the rest are dropped (`too-many-diagnostics`).
    pub max_diagnostics: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        ParseLimits {
            max_blocks: MAX_BLOCKS,
            max_diagnostics: MAX_DIAGNOSTICS,
        }
    }
}

/// Like [parse] with the limits given.
pub fn parse_with_limits(text: &str, dialect: Dialect, limits: &ParseLimits) -> Program {
    let mut blocks = Vec::new();
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut cut = false;
    // a carriage return ends a line for a controller wherever it stands: what follows it is not part of a comment
    let lines = text
        .split_terminator('\n')
        .flat_map(|l| l.strip_suffix('\r').unwrap_or(l).split('\r'));
    for (index, raw) in lines.enumerate() {
        let line = index + 1;
        let raw = raw.trim_start_matches('\u{feff}');
        let mut raw = raw;
        if raw.chars().count() > MAX_LINE_CHARS {
            let cut = raw
                .char_indices()
                .nth(MAX_LINE_CHARS)
                .map(|(i, _)| i)
                .unwrap_or(raw.len());
            // losing the end of a comment changes nothing; losing code means that part of the program was not looked at
            let severity = if code_past(raw, cut) {
                Severity::Error
            } else {
                Severity::Warning
            };
            raw = raw.get(..cut).unwrap_or(raw);
            diagnostics.push(Diagnostic {
                line,
                severity,
                code: codes::LINE_TOO_LONG,
                message: format!("the line is longer than {MAX_LINE_CHARS} characters and was cut"),
            });
        }
        if blocks.len() >= limits.max_blocks {
            diagnostics.push(Diagnostic::new(
                line,
                Severity::Error,
                codes::PROGRAM_TOO_LONG,
                format!(
                    "the program has more than {} blocks: the rest was not read",
                    limits.max_blocks
                ),
            ));
            break;
        }
        if let Some(block) = parse_line(raw, line, dialect, &mut diagnostics) {
            blocks.push(block);
        }
        if cut {
            diagnostics.truncate(limits.max_diagnostics + 1);
        } else if diagnostics.len() > limits.max_diagnostics {
            diagnostics.truncate(limits.max_diagnostics);
            diagnostics.push(Diagnostic::new(
                line,
                Severity::Error,
                codes::TOO_MANY_DIAGNOSTICS,
                format!(
                    "more than {} diagnostics: the rest are not kept",
                    limits.max_diagnostics
                ),
            ));
            cut = true;
        }
    }
    Program {
        dialect,
        blocks,
        diagnostics,
    }
}

/// Whether anything outside the comments of `line` stands at or after the byte `from`.
fn code_past(line: &str, from: usize) -> bool {
    let mut open = false;
    for (at, c) in line.char_indices() {
        match c {
            ';' if !open => return false,
            '(' if !open => open = true,
            ')' if open => open = false,
            _ if open || c.is_whitespace() => {}
            _ if at >= from => return true,
            _ => {}
        }
    }
    false
}

/// The characters of a comment that GRBL obeys at once if the comment reaches the controller: `!`, `~`, `?` and the control characters. Accented
/// letters and the other characters outside ASCII are taken in a comment: a sender has to drop or clean the comments, because GRBL takes some of
/// their bytes as commands too. Outside a comment every one of them is refused as a character left out of the code.
fn is_realtime(c: char) -> bool {
    matches!(c, '!' | '~' | '?') || (c.is_control() && c != '\t')
}

/// The `$` commands of GRBL that only ask. Every other one moves the machine or changes how it behaves.
const GRBL_QUESTIONS: [&str; 6] = ["$", "$$", "$#", "$G", "$I", "$N"];

/// The code of a line without its comments, and the text of the comments.
struct SplitLine {
    code: Vec<char>,
    comment: Option<String>,
    unclosed: bool,
    /// A character the controller obeys at once stands inside a comment.
    realtime_comment: bool,
}

/// Takes the comments out of a line. With `join` (GRBL) the blanks of the code are taken out too: the controller reads `S1 000` as `S1000`.
fn split_line(body: &str, join: bool) -> SplitLine {
    let mut out = SplitLine {
        code: Vec::new(),
        comment: None,
        unclosed: false,
        realtime_comment: false,
    };
    let keep = |comment: &mut Option<String>, text: String| {
        *comment = Some(match comment.take() {
            Some(old) => format!("{old} {text}"),
            None => text,
        });
    };
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == ';' {
            let text: String = chars.by_ref().collect();
            out.realtime_comment |= text.chars().any(is_realtime);
            keep(&mut out.comment, text.trim().to_string());
        } else if c == '(' {
            let mut text = String::new();
            let mut closed = false;
            for d in chars.by_ref() {
                if d == ')' {
                    closed = true;
                    break;
                }
                text.push(d);
            }
            out.unclosed |= !closed;
            out.realtime_comment |= text.chars().any(is_realtime);
            keep(&mut out.comment, text);
            // where the blanks count, a comment keeps two words apart
            if !join {
                out.code.push(' ');
            }
        } else if !(join && (c == ' ' || c == '\t')) {
            out.code.push(c);
        }
    }
    out
}

fn parse_line(
    raw: &str,
    line: usize,
    dialect: Dialect,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Block> {
    let mut body = raw.trim();
    if let Some(after) = body.strip_prefix('/') {
        diagnostics.push(Diagnostic {
            line,
            severity: Severity::Info,
            code: codes::BLOCK_DELETE_READ,
            message: "a block-delete line was read as if the switch were off".to_string(),
        });
        body = after.trim_start();
    }
    // a line with only % starts or ends the program in Mach3 and PlanetCNC: it is not a character to complain about
    if body == "%" {
        return None;
    }
    // Marlin: a checksum after '*' is verified and cut
    if dialect == Dialect::Marlin {
        // a '*' after a ';' belongs to the comment, not to a checksum
        let star = body.find('*').filter(|&s| match body.find(';') {
            Some(c) => s < c,
            None => true,
        });
        if let Some((content, sum)) = star.and_then(|_| body.split_once('*')) {
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
                    code: codes::CHECKSUM_MISMATCH,
                    message: "the checksum of the line does not match".to_string(),
                }),
                Err(_) => diagnostics.push(Diagnostic {
                    line,
                    severity: Severity::Error,
                    code: codes::CHECKSUM_INVALID,
                    message: "the checksum of the line is not a number".to_string(),
                }),
            }
            body = content;
        }
    }
    if dialect == Dialect::Marlin {
        if let Some(block) = text_command(body, line) {
            return Some(block);
        }
    }
    let grbl = dialect == Dialect::Grbl;
    let split = split_line(body, grbl);
    if split.unclosed {
        diagnostics.push(Diagnostic {
            line,
            severity: Severity::Warning,
            code: codes::COMMENT_UNCLOSED,
            message: "a comment in parentheses is not closed".to_string(),
        });
    }
    if grbl && split.realtime_comment {
        diagnostics.push(Diagnostic {
            line,
            severity: Severity::Warning,
            code: codes::REALTIME_CHARACTER,
            message: "a comment holds a character that GRBL obeys at once if it reaches the controller (!, ~, ? or a control character)".to_string(),
        });
    }
    let comment = split.comment;
    let chars = split.code;
    if grbl && chars.first() == Some(&'$') {
        let command: String = chars.iter().collect::<String>().to_ascii_uppercase();
        let asks = GRBL_QUESTIONS.contains(&command.as_str());
        let (severity, message) = if asks {
            (Severity::Info, "a GRBL question ($) was left out")
        } else {
            (Severity::Error, "a GRBL system command ($) is not part of a program: it can move the machine or change its settings")
        };
        diagnostics.push(Diagnostic {
            line,
            severity,
            code: codes::SYSTEM_COMMAND_IGNORED,
            message: message.to_string(),
        });
        return None;
    }
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c.is_ascii_alphabetic() {
            let letter = c.to_ascii_uppercase();
            i += 1;
            let start = i;
            while i < chars.len() && matches!(chars[i], '0'..='9' | '.' | '-' | '+') {
                i += 1;
            }
            let number: String = chars[start..i].iter().collect();
            match parse_number(&number) {
                Ok(value) => words.push(Word { letter, value }),
                Err(why) => diagnostics.push(Diagnostic {
                    line,
                    severity: Severity::Error,
                    code: match why {
                        NumberError::TooLarge => codes::NUMBER_TOO_LARGE,
                        NumberError::Invalid => codes::WORD_NUMBER_INVALID,
                    },
                    message: format!("the word '{letter}{number}' has no valid number"),
                }),
            }
        } else {
            diagnostics.push(Diagnostic {
                line,
                severity: Severity::Error,
                code: codes::CHARACTER_IGNORED,
                message: format!(
                    "the character '{}' could not be read and was left out",
                    c.escape_default()
                ),
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

/// The Marlin commands whose argument is text (M117 Hello, M23 file.gco): the line is one word and the text is kept as its comment, not read as words.
fn text_command(body: &str, line: usize) -> Option<Block> {
    let mut rest = body.trim_start();
    // an optional line number
    if rest.starts_with(['N', 'n']) {
        rest = rest
            .get(1..)?
            .trim_start_matches(|c: char| c.is_ascii_digit());
        rest = rest.trim_start();
    }
    if !rest.starts_with(['M', 'm']) {
        return None;
    }
    let digits: String = rest
        .get(1..)?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let code: u32 = digits.parse().ok()?;
    if ![23, 28, 29, 32, 117, 118, 550, 551].contains(&code) {
        return None;
    }
    let text = rest.get(1 + digits.len()..)?;
    // M1170 is another command: the code ends at a space or at the end
    if text.chars().next().is_some_and(|c| !c.is_whitespace()) {
        return None;
    }
    let text = text.trim();
    Some(Block {
        line,
        words: vec![Word {
            letter: 'M',
            value: f64::from(code),
        }],
        comment: (!text.is_empty()).then(|| text.to_string()),
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
    /// True after `G80` until the next motion code: axis words alone move nothing.
    cancelled: bool,
    seen_move: bool,
    /// False after G18 or G19: arcs are not interpreted.
    plane_xy: bool,
    /// A spindle speed of 0 means off (the laser mode of GRBL: `M3 S0` and `M4 S0` do not burn).
    s0_is_off: bool,
    /// True only for [`interpret_coordinates`]: `G92`, `G28`, `G30`, `G53` and `G54`..`G59` are followed.
    track: bool,
    coord: CoordinateOptions,
    /// The index of the active work coordinate system (0 is G54).
    work: usize,
    /// The stored `G92` offset, in millimetres.
    g92: Point3,
    /// Whether the stored `G92` offset is applied (`G92.2` suspends it, `G92.3` applies it again).
    g92_on: bool,
}

/// The largest offset of `G92` that [`CoordinateOptions::default`] accepts, in millimetres.
pub const DEFAULT_MAX_OFFSET_MM: f64 = 100_000.0;

/// What [`interpret_coordinates`] needs to follow `G92`, `G28`, `G30`, `G53` and `G54`..`G59`.
///
/// Nothing here is invented: the default has no reference position and every work coordinate system at the machine
/// origin. Fill it from the machine configuration of your program. All positions are machine coordinates in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateOptions {
    /// Where `G28` goes (`None`: not configured, `G28` is told with `reference-position-unset` and moves nothing).
    pub reference_g28: Option<Point3>,
    /// Where `G30` goes (`None`: not configured).
    pub reference_g30: Option<Point3>,
    /// The origin of each work coordinate system `G54`..`G59` in machine coordinates.
    pub work_offsets: [Point3; 6],
    /// The largest absolute value a `G92` offset may take, in millimetres (`offset-out-of-range` above it).
    pub max_offset_mm: f64,
}

impl Default for CoordinateOptions {
    fn default() -> Self {
        CoordinateOptions {
            reference_g28: None,
            reference_g30: None,
            work_offsets: [Point3::default(); 6],
            max_offset_mm: DEFAULT_MAX_OFFSET_MM,
        }
    }
}

/// The coordinate offsets in force at the end of a program walked by [`interpret_coordinates`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateState {
    /// The active work coordinate system: 54 for `G54`, up to 59.
    pub work_system: u8,
    /// The origin of the active work coordinate system in machine coordinates.
    pub work_offset: Point3,
    /// The stored `G92` offset (zero after `G92.1`).
    pub g92_offset: Point3,
    /// Whether the `G92` offset is applied (it is not after `G92.2`).
    pub g92_active: bool,
    /// What is added to a work coordinate to get the machine coordinate: the work offset plus the `G92` offset when applied.
    pub offset: Point3,
}

impl Default for CoordinateState {
    fn default() -> Self {
        CoordinateState {
            work_system: 54,
            work_offset: Point3::default(),
            g92_offset: Point3::default(),
            g92_active: false,
            offset: Point3::default(),
        }
    }
}

/// The result of [`interpret_coordinates`]: the trace, with its positions in machine coordinates, and the offsets at the end.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateTrace {
    /// The motions and the notices, as [`interpret`] gives them.
    pub trace: Trace,
    /// The offsets in force at the end of the program.
    pub state: CoordinateState,
}

/// How [`interpret_with`] reads the state of the spindle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InterpretOptions {
    /// With `true`, `M3` or `M4` with a spindle speed of 0 do not count as the spindle (or laser) being on: in the laser mode of GRBL the power is the S word. The default is `false`.
    pub s0_is_off: bool,
}

/// Walks a program, keeping the modal state, and returns its motions.
pub fn interpret(program: &Program) -> Trace {
    interpret_with(program, &InterpretOptions::default())
}

/// Like [`interpret`] with the options of your own.
pub fn interpret_with(program: &Program, options: &InterpretOptions) -> Trace {
    walk(program, options, None).0
}

/// Like [`interpret_with`], and also follows the commands that move the origin: `G92` (and `G92.1`, `G92.2`, `G92.3`), `G28` and
/// `G30` (to the reference positions of `coordinates`), `G53` (machine coordinates for that block) and `G54`..`G59` (the
/// work offsets of `coordinates`).
///
/// The positions of the trace are machine coordinates in millimetres; the machine starts at X0 Y0 Z0. A program that uses none
/// of these commands gives the same trace as [`interpret_with`]. What cannot be followed is told with a diagnostic
/// (`reference-position-unset`, `g53-motion-unsupported`, `g92-axes-missing`, `offset-out-of-range`).
pub fn interpret_coordinates(
    program: &Program,
    options: &InterpretOptions,
    coordinates: &CoordinateOptions,
) -> CoordinateTrace {
    let (trace, state) = walk(program, options, Some(coordinates));
    CoordinateTrace { trace, state }
}

fn walk(
    program: &Program,
    options: &InterpretOptions,
    coordinates: Option<&CoordinateOptions>,
) -> (Trace, CoordinateState) {
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
        cancelled: false,
        seen_move: false,
        plane_xy: true,
        s0_is_off: options.s0_is_off,
        track: coordinates.is_some(),
        coord: coordinates.copied().unwrap_or_default(),
        work: 0,
        g92: Point3::default(),
        g92_on: false,
    };
    let mut trace = Trace {
        diagnostics: program.diagnostics.clone(),
        ..Trace::default()
    };
    for block in &program.blocks {
        if trace.ended {
            diag(
                &mut trace,
                block.line,
                Severity::Error,
                codes::TEXT_AFTER_END,
                "the program went on after M2/M30: a controller runs what it is sent, and it was not looked at",
            );
            break;
        }
        run_block(block, &mut state, &mut trace, program.dialect);
    }
    let end = coordinate_state(&state);
    (trace, end)
}

fn add(a: Point3, b: Point3) -> Point3 {
    Point3 {
        x: a.x + b.x,
        y: a.y + b.y,
        z: a.z + b.z,
    }
}

/// What is added to a work coordinate to get the machine one (zero when the coordinates are not followed).
fn active_offset(st: &State) -> Point3 {
    if !st.track {
        return Point3::default();
    }
    let work = st.coord.work_offsets[st.work];
    if st.g92_on {
        add(work, st.g92)
    } else {
        work
    }
}

fn coordinate_state(st: &State) -> CoordinateState {
    CoordinateState {
        work_system: 54 + st.work as u8,
        work_offset: st.coord.work_offsets[st.work],
        g92_offset: st.g92,
        g92_active: st.g92_on,
        offset: active_offset(st),
    }
}

/// A coordinate shifted by an offset; a zero offset leaves it as it is (a negative zero stays one).
fn shifted(value: f64, offset: f64) -> f64 {
    if offset == 0.0 {
        value
    } else {
        value + offset
    }
}

/// What a G word does to the coordinates when they are followed.
enum CoordWord {
    /// Not a word about coordinates: the usual rules read it.
    Unhandled,
    /// Done.
    Handled,
    /// Done, and the axis words of the block are not a move.
    SkipMotion,
    /// `G53`: the move of this block is in machine coordinates.
    Machine,
    /// `G28` (false) or `G30` (true): go to its reference position after the move through the point of the block.
    Reference(bool),
}

fn word_mm(st: &State, v: f64) -> f64 {
    if st.inches {
        v * MM_PER_INCH
    } else {
        v
    }
}

fn coordinate_word(code: i64, block: &Block, st: &mut State, trace: &mut Trace) -> CoordWord {
    match code {
        540 | 550 | 560 | 570 | 580 | 590 => {
            st.work = ((code - 540) / 10) as usize;
            CoordWord::Handled
        }
        920 => {
            set_g92(block, st, trace);
            CoordWord::SkipMotion
        }
        921 => {
            st.g92 = Point3::default();
            st.g92_on = false;
            CoordWord::SkipMotion
        }
        922 => {
            st.g92_on = false;
            CoordWord::SkipMotion
        }
        923 => {
            st.g92_on = true;
            CoordWord::SkipMotion
        }
        530 => CoordWord::Machine,
        280 | 300 => {
            let reference = if code == 280 {
                st.coord.reference_g28
            } else {
                st.coord.reference_g30
            };
            let usable =
                reference.is_some_and(|p| p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
            if usable {
                CoordWord::Reference(code == 300)
            } else {
                diag(
                    trace,
                    block.line,
                    Severity::Warning,
                    codes::REFERENCE_POSITION_UNSET,
                    format!(
                        "G{} was read but no reference position is configured for it: it moved nothing and the positions after it may be wrong",
                        code / 10
                    ),
                );
                CoordWord::SkipMotion
            }
        }
        _ => CoordWord::Unhandled,
    }
}

/// `G92`: the work position of the axes written becomes the value given, without moving.
fn set_g92(block: &Block, st: &mut State, trace: &mut Trace) {
    let work = st.coord.work_offsets[st.work];
    let limit = st.coord.max_offset_mm;
    let axes = [
        ('X', st.position.x, work.x),
        ('Y', st.position.y, work.y),
        ('Z', st.position.z, work.z),
    ];
    let mut next = [st.g92.x, st.g92.y, st.g92.z];
    let mut any_axis = false;
    let mut accepted = false;
    for (slot, (letter, position, origin)) in next.iter_mut().zip(axes) {
        let Some(v) = block.get(letter) else {
            continue;
        };
        any_axis = true;
        let candidate = position - origin - word_mm(st, v);
        if candidate.abs() > limit {
            diag(
                trace,
                block.line,
                Severity::Error,
                codes::OFFSET_OUT_OF_RANGE,
                format!("the G92 offset of {letter} would be {candidate:.3} mm, more than the limit of {limit} mm: it was not changed"),
            );
        } else {
            *slot = candidate;
            accepted = true;
        }
    }
    st.g92 = Point3 {
        x: next[0],
        y: next[1],
        z: next[2],
    };
    if accepted {
        st.g92_on = true;
    } else if !any_axis
        && !block
            .words
            .iter()
            .any(|w| matches!(w.letter, 'A' | 'B' | 'C' | 'E' | 'U' | 'V' | 'W'))
    {
        diag(
            trace,
            block.line,
            Severity::Warning,
            codes::G92_AXES_MISSING,
            "G92 has no axis word: the offset was not changed",
        );
    }
}

/// `G28` / `G30`: the move through the point of the block (not in Marlin, where the axes only say which ones go home), then the
/// axes written (all of them when there is none) go to the reference position.
fn run_reference(
    block: &Block,
    st: &mut State,
    trace: &mut Trace,
    g30: bool,
    dialect: Dialect,
    scale: impl Fn(&State, f64) -> f64,
) {
    let reference = if g30 {
        st.coord.reference_g30
    } else {
        st.coord.reference_g28
    };
    let Some(home) = reference else {
        return;
    };
    let listed = |letter: char| block.get(letter).is_some();
    let any_listed = listed('X') || listed('Y') || listed('Z');
    if any_listed && dialect != Dialect::Marlin {
        run_motion(block, st, trace, 0, false, scale);
    }
    let from = st.position;
    let pick = |letter: char, current: f64, target: f64| {
        if any_listed && !listed(letter) {
            current
        } else {
            target
        }
    };
    let to = Point3 {
        x: pick('X', from.x, home.x),
        y: pick('Y', from.y, home.y),
        z: pick('Z', from.z, home.z),
    };
    if to != from {
        st.seen_move = true;
        st.position = to;
        push(block, st, trace, Motion::Rapid, from, to);
    }
}

fn diag(
    trace: &mut Trace,
    line: usize,
    severity: Severity,
    code: &'static str,
    message: impl Into<String>,
) {
    if trace.diagnostics.len() > MAX_DIAGNOSTICS {
        return;
    }
    if trace.diagnostics.len() == MAX_DIAGNOSTICS {
        trace.diagnostics.push(Diagnostic::new(
            line,
            Severity::Error,
            codes::TOO_MANY_DIAGNOSTICS,
            format!("more than {MAX_DIAGNOSTICS} diagnostics: the rest are not kept"),
        ));
        return;
    }
    trace
        .diagnostics
        .push(Diagnostic::new(line, severity, code, message));
}

/// A G word as ten times its number (`G91.1` is 911); a number with more decimals is no code at all.
fn g_code(v: f64) -> i64 {
    let tenths = (v * 10.0).round();
    if (v * 10.0 - tenths).abs() < 1e-6 && v >= 0.0 {
        tenths as i64
    } else {
        -1
    }
}

/// What a controller refuses before it runs anything of the block: a word written twice, and two codes that cannot both hold.
fn check_words(block: &Block, trace: &mut Trace, dialect: Dialect) {
    let mut seen = [false; 26];
    let mut twice = None;
    for w in &block.words {
        if matches!(w.letter, 'G' | 'M') || !w.letter.is_ascii_uppercase() {
            continue;
        }
        let at = usize::from(w.letter as u8 - b'A');
        if std::mem::replace(&mut seen[at], true) {
            twice = Some(w.letter);
        }
    }
    if let Some(letter) = twice {
        diag(
            trace,
            block.line,
            Severity::Error,
            codes::WORD_REPEATED,
            format!("the word '{letter}' is written twice: the controller refuses the line"),
        );
    }
    let count = |letter: char, group: &[i64]| {
        block
            .words
            .iter()
            .filter(|w| w.letter == letter && group.contains(&g_code(w.value)))
            .count()
    };
    // Marlin takes one command a line, so its groups are not those of a CNC controller
    if dialect != Dialect::Marlin
        && (count('G', &[0, 10, 20, 30, 800]) > 1
            || count('G', &[200, 210]) > 1
            || count('G', &[900, 910]) > 1
            || count('G', &[170, 180, 190]) > 1
            || count('M', &[30, 40, 50]) > 1)
    {
        diag(
            trace,
            block.line,
            Severity::Error,
            codes::WORD_REPEATED,
            "two codes of the same group on one line: the controller refuses the line",
        );
    }
}

fn run_block(block: &Block, st: &mut State, trace: &mut Trace, dialect: Dialect) {
    let scale = |st: &State, v: f64| if st.inches { v * MM_PER_INCH } else { v };
    // the units of the block come first, wherever G20/G21 are written: the feed of `F10 G20` is in inches (as the controller reads it)
    for word in &block.words {
        if word.letter == 'G' {
            match g_code(word.value) {
                200 => st.inches = true,
                210 => st.inches = false,
                _ => {}
            }
        }
    }
    // a block with a feature that is not supported does not move anything
    let mut skip_motion = false;
    // G53: the move of this block is in machine coordinates
    let mut machine = false;
    // G28 (false) or G30 (true) with a reference position
    let mut reference: Option<bool> = None;
    check_words(block, trace, dialect);
    // 1. the modal words, in the order of the block
    for word in &block.words {
        let v = word.value;
        match word.letter {
            'G' => {
                let code = g_code(v); // G91.1 -> 911
                if st.track {
                    match coordinate_word(code, block, st, trace) {
                        CoordWord::Unhandled => {}
                        CoordWord::Handled => continue,
                        CoordWord::SkipMotion => {
                            skip_motion = true;
                            continue;
                        }
                        CoordWord::Machine => {
                            machine = true;
                            continue;
                        }
                        CoordWord::Reference(g30) => {
                            skip_motion = true;
                            reference = Some(g30);
                            continue;
                        }
                    }
                }
                match code {
                    0 | 10 | 20 | 30 => {
                        st.motion = Some((code / 10) as u32);
                        st.cancelled = false;
                    }
                    800 => {
                        st.motion = None;
                        st.cancelled = true;
                    }
                    930 | 950 => diag(
                        trace,
                        block.line,
                        Severity::Error,
                        codes::FEED_MODE_UNSUPPORTED,
                        format!(
                            "G{v} changes what F means: the speeds of this program cannot be told"
                        ),
                    ),
                    430 | 431 | 440 => {
                        // G43.1 takes the axis word as the offset: it is not a move
                        skip_motion |= code == 431;
                        diag(
                            trace,
                            block.line,
                            Severity::Warning,
                            codes::POSITION_UNTRACKED,
                            format!("the tool length offset of G{v} is not tracked: the heights after it may be wrong"),
                        );
                    }
                    200 => st.inches = true,
                    210 => st.inches = false,
                    900 => st.absolute = true,
                    910 => st.absolute = false,
                    901 => st.absolute_arc_centre = true,
                    911 => st.absolute_arc_centre = false,
                    170 => st.plane_xy = true,
                    180 | 190 => {
                        st.plane_xy = false;
                        diag(
                            trace,
                            block.line,
                            Severity::Error,
                            codes::PLANE_UNSUPPORTED,
                            "only the XY plane (G17) is supported for arcs: the arcs after G18/G19 are drawn as lines",
                        );
                    }
                    40 | 400 | 540 | 550 | 560 | 570 | 580 | 590 | 921 | 922 | 940 | 500 | 610
                    | 640 | 490 => {}
                    // The words of these blocks are not a move of the modal motion: reading them as one would invent a cut.
                    // Their effect on the position is not tracked, so what comes after may be offset.
                    280 | 300 | 920 | 530 | 100 | 380..=395 => {
                        skip_motion = true;
                        diag(
                            trace,
                            block.line,
                            Severity::Warning,
                            codes::POSITION_UNTRACKED,
                            format!(
                                "G{} was read but its effect on the position is not tracked: the positions after it may be wrong",
                                v
                            ),
                        );
                    }
                    410 | 420 => {
                        skip_motion = true;
                        diag(
                            trace,
                            block.line,
                            Severity::Error,
                            codes::CUTTER_COMPENSATION_UNSUPPORTED,
                            "cutter radius compensation (G41/G42) is not supported",
                        );
                    }
                    c if (810..=890).contains(&c) => {
                        skip_motion = true;
                        diag(
                            trace,
                            block.line,
                            Severity::Error,
                            codes::CANNED_CYCLE_UNSUPPORTED,
                            format!("canned cycles (G{v}) are not supported"),
                        );
                    }
                    _ => {
                        // with axis words the unknown code may say what they mean: reading them as the modal move would invent one
                        let takes_axes = block
                            .words
                            .iter()
                            .any(|w| matches!(w.letter, 'X' | 'Y' | 'Z'));
                        skip_motion |= takes_axes;
                        let severity = if takes_axes {
                            Severity::Error
                        } else {
                            Severity::Warning
                        };
                        diag(
                            trace,
                            block.line,
                            severity,
                            codes::UNKNOWN_G_CODE,
                            format!("G{v} is not known and was ignored"),
                        );
                    }
                }
            }
            'M' if v.fract() != 0.0 || v < 0.0 => diag(
                trace,
                block.line,
                Severity::Error,
                codes::UNKNOWN_M_CODE,
                format!("M{v} is not known and was ignored"),
            ),
            'M' => match v as i64 {
                0 | 1 => {}
                2 | 30 => trace.ended = true,
                3 | 4 => st.spindle_on = true,
                5 => st.spindle_on = false,
                6 => {}
                7..=9 => {}
                104 | 109 | 140 | 190 | 106 | 107 | 23 | 28 | 29 | 32 | 117 | 118 | 550 | 551
                    if dialect == Dialect::Marlin => {}
                _ => diag(
                    trace,
                    block.line,
                    Severity::Warning,
                    codes::UNKNOWN_M_CODE,
                    format!("M{v} is not known and was ignored"),
                ),
            },
            'F' => {
                if v < 0.0 {
                    diag(
                        trace,
                        block.line,
                        Severity::Error,
                        codes::NEGATIVE_FEED,
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
                        codes::NEGATIVE_SPINDLE,
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
                        codes::TOOL_NUMBER_INVALID,
                        "a tool number that is not a whole number was ignored",
                    );
                }
            }
            'E' if dialect == Dialect::Grbl => diag(
                trace,
                block.line,
                Severity::Error,
                codes::UNKNOWN_WORD,
                "GRBL has no word 'E': it refuses the line",
            ),
            'P' | 'L' | 'N' | 'H' | 'D' | 'E' | 'R' | 'X' | 'Y' | 'Z' | 'I' | 'J' | 'K' | 'A'
            | 'B' | 'C' | 'U' | 'V' | 'W' | 'Q' => {}
            other => diag(
                trace,
                block.line,
                Severity::Warning,
                codes::UNKNOWN_WORD,
                format!("the word '{other}' is not known and was ignored"),
            ),
        }
    }
    // 2. the motion of the block
    if let Some(g30) = reference {
        run_reference(block, st, trace, g30, dialect, scale);
        return;
    }
    let has_axis = block
        .words
        .iter()
        .any(|w| matches!(w.letter, 'X' | 'Y' | 'Z'));
    if !has_axis || skip_motion {
        return;
    }
    let code = match st.motion {
        Some(code) => code,
        // G80 took the motion mode away
        None if st.cancelled => {
            diag(
                trace,
                block.line,
                Severity::Error,
                codes::UNKNOWN_WORD,
                "axis words after G80 with no new G0/G1/G2/G3: the controller refuses the line",
            );
            return;
        }
        None => {
            if !st.seen_move {
                diag(
                    trace,
                    block.line,
                    Severity::Warning,
                    codes::IMPLIED_RAPID,
                    "an axis word comes before any G0/G1/G2/G3: it was taken as a rapid (G0)",
                );
            }
            st.motion = Some(0);
            0
        }
    };
    if machine && code > 1 {
        diag(
            trace,
            block.line,
            Severity::Error,
            codes::G53_MOTION_UNSUPPORTED,
            "G53 moves only with G0 or G1: the block moved nothing",
        );
        return;
    }
    run_motion(block, st, trace, code, machine, scale);
}

fn run_motion(
    block: &Block,
    st: &mut State,
    trace: &mut Trace,
    code: u32,
    machine: bool,
    scale: impl Fn(&State, f64) -> f64,
) {
    st.seen_move = true;
    let from = st.position;
    // G53 is in machine coordinates, and then so is the distance mode of the block: absolute
    let offset = if machine {
        Point3::default()
    } else {
        active_offset(st)
    };
    let target = |letter: char, current: f64, shift: f64, st: &State| -> f64 {
        match block.get(letter) {
            Some(v) => {
                let mm = scale(st, v);
                if st.absolute || machine {
                    shifted(mm, shift)
                } else {
                    current + mm
                }
            }
            None => current,
        }
    };
    let to = Point3 {
        x: target('X', from.x, offset.x, st),
        y: target('Y', from.y, offset.y, st),
        z: target('Z', from.z, offset.z, st),
    };
    let motion = match code {
        0 => Motion::Rapid,
        1 => Motion::Linear,
        2 | 3 => {
            let (i, j, r) = (block.get('I'), block.get('J'), block.get('R'));
            let centre = if !st.plane_xy {
                diag(
                    trace,
                    block.line,
                    Severity::Error,
                    codes::PLANE_UNSUPPORTED,
                    "an arc outside the XY plane was drawn as a straight line",
                );
                None
            } else if i.is_some() || j.is_some() {
                let i_mm = scale(st, i.unwrap_or(0.0));
                let j_mm = scale(st, j.unwrap_or(0.0));
                let c = if st.absolute_arc_centre {
                    (shifted(i_mm, offset.x), shifted(j_mm, offset.y))
                } else {
                    (from.x + i_mm, from.y + j_mm)
                };
                // the centre must be as far from the start as from the end (a controller refuses an arc that is not)
                let (r0, r1) = (
                    (from.x - c.0).hypot(from.y - c.1),
                    (to.x - c.0).hypot(to.y - c.1),
                );
                let delta = (r0 - r1).abs();
                if delta > 0.005 && (delta > 0.5 || delta > 0.001 * r0) {
                    diag(
                        trace,
                        block.line,
                        Severity::Error,
                        codes::ARC_RADIUS_MISMATCH,
                        format!("the arc is {r0:.3} mm from its centre at the start and {r1:.3} mm at the end"),
                    );
                }
                Some(c)
            } else if let Some(r) = r {
                match centre_from_radius(from, to, scale(st, r), code == 2) {
                    Some(c) => Some(c),
                    None => {
                        diag(trace, block.line, Severity::Error, codes::ARC_RADIUS_UNREACHABLE, "an arc with a radius (R) that does not reach from its start to its end was drawn as a straight line");
                        None
                    }
                }
            } else {
                diag(
                    trace,
                    block.line,
                    Severity::Error,
                    codes::ARC_CENTRE_MISSING,
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
            codes::FEED_MISSING,
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
        spindle_on: st.spindle_on && !(st.s0_is_off && st.speed == 0.0),
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
    let mut reached = vec![from, to];
    if let Motion::ArcCw { center } | Motion::ArcCcw { center } = motion {
        reached.extend(arc_extremes(
            from,
            to,
            center,
            matches!(motion, Motion::ArcCw { .. }),
        ));
    }
    for p in reached {
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

/// The points of an arc where it reaches the most extreme X or Y (the four points of its circle that lie within its sweep): the arc bulges out
/// past its two ends, and the box of the motion has to hold that.
fn arc_extremes(from: Point3, to: Point3, center: (f64, f64), clockwise: bool) -> Vec<Point3> {
    let r = (from.x - center.0).hypot(from.y - center.1);
    if r == 0.0 {
        return Vec::new();
    }
    let tau = std::f64::consts::TAU;
    let a0 = (from.y - center.1).atan2(from.x - center.0);
    let a1 = (to.y - center.1).atan2(to.x - center.0);
    // the angle covered, counter-clockwise positive; a start that is also the end is a whole circle
    let mut sweep = (a1 - a0).rem_euclid(tau);
    if clockwise {
        sweep = if sweep == 0.0 { -tau } else { sweep - tau };
    } else if sweep == 0.0 {
        sweep = tau;
    }
    let mut out = Vec::new();
    for k in 0..4 {
        let cardinal = f64::from(k) * std::f64::consts::FRAC_PI_2;
        // how far from the start, going the way of the arc, this point of the circle is
        let away = if sweep >= 0.0 {
            (cardinal - a0).rem_euclid(tau)
        } else {
            -((a0 - cardinal).rem_euclid(tau))
        };
        if away.abs() <= sweep.abs() {
            out.push(Point3 {
                x: center.0 + r * cardinal.cos(),
                y: center.1 + r * cardinal.sin(),
                z: from.z,
            });
        }
    }
    out
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
