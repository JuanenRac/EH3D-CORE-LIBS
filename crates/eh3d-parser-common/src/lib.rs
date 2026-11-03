//! # eh3d-parser-common
//!
//! The pieces the three readers (`eh3d-parser-gerber`, `eh3d-parser-excellon` and
//! `eh3d-parser-gcode`) share: [`Severity`], [`Diagnostic`] with a stable [`codes`] entry, [`Units`]
//! and the rule for a file that does not declare them, [`decode_coordinate`], the bounded number
//! parsers and [`decode_input`], which turns bytes into text with a size limit.
//!
//! The readers re-export the types they used to define, so a program that depends on a reader does
//! not need this crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::string_slice)]

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
    /// A stable identifier of the kind of notice (see [`codes`]); filter on this, not on the text.
    pub code: &'static str,
    /// What happened, in English.
    pub message: String,
}

impl Diagnostic {
    /// A diagnostic from its parts.
    pub fn new(
        line: usize,
        severity: Severity,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Diagnostic {
            line,
            severity,
            code,
            message: message.into(),
        }
    }
}

/// The stable codes of the diagnostics. Each one is documented in `docs/DIAGNOSTICS.md`.
pub mod codes {
    macro_rules! codes {
        ($($name:ident = $text:literal,)*) => {
            $(
                #[doc = concat!("`", $text, "`")]
                pub const $name: &str = $text;
            )*
            /// Every code, in the order of the documentation table.
            pub const ALL: &[&str] = &[$($text),*];
        };
    }
    codes! {
        // shared by several readers
        UNITS_MISSING = "units-missing",
        NUMBER_TOO_LARGE = "number-too-large",
        COORDINATE_INVALID = "coordinate-invalid",
        UNKNOWN_G_CODE = "unknown-g-code",
        UNKNOWN_M_CODE = "unknown-m-code",
        UNKNOWN_WORD = "unknown-word",
        END_MISSING = "end-missing",
        INCREMENTAL_UNSUPPORTED = "incremental-unsupported",
        DEPRECATED_COMMAND = "deprecated-command",
        COMMAND_IGNORED = "command-ignored",
        TEXT_AFTER_END = "text-after-end",
        INPUT_NOT_UTF8 = "input-not-utf8",
        TOO_MANY_DIAGNOSTICS = "too-many-diagnostics",
        COMMAND_UNSUPPORTED = "command-unsupported",
        UNKNOWN_COMMAND = "unknown-command",
        GEOMETRY_COMMAND_UNSUPPORTED = "geometry-command-unsupported",
        UNKNOWN_HEADER_LINE = "unknown-header-line",
        POSITION_UNTRACKED = "position-untracked",
        // Gerber
        EXTENDED_UNCLOSED = "extended-unclosed",
        BLOCK_UNCLOSED = "block-unclosed",
        REGION_UNCLOSED = "region-unclosed",
        REGION_NESTED = "region-nested",
        REGION_NOT_OPEN = "region-not-open",
        MACRO_NAME_MISSING = "macro-name-missing",
        UNITS_UNKNOWN = "units-unknown",
        POLARITY_UNKNOWN = "polarity-unknown",
        FORMAT_ZERO_OMISSION_INVALID = "format-zero-omission-invalid",
        FORMAT_TRAILING_DEPRECATED = "format-trailing-deprecated",
        FORMAT_MODE_INVALID = "format-mode-invalid",
        FORMAT_MALFORMED = "format-malformed",
        FORMAT_AXES_DIFFER = "format-axes-differ",
        FORMAT_DIGITS_INVALID = "format-digits-invalid",
        COORDINATE_BEFORE_FORMAT = "coordinate-before-format",
        APERTURE_NUMBER_INVALID = "aperture-number-invalid",
        APERTURE_NUMBER_RESERVED = "aperture-number-reserved",
        APERTURE_PARAMETER_INVALID = "aperture-parameter-invalid",
        APERTURE_PARAMETERS_MISSING = "aperture-parameters-missing",
        APERTURE_POLYGON_VERTICES = "aperture-polygon-vertices",
        APERTURE_MACRO_UNDEFINED = "aperture-macro-undefined",
        APERTURE_TEMPLATE_MISSING = "aperture-template-missing",
        APERTURE_SIZE_NEGATIVE = "aperture-size-negative",
        APERTURE_REDEFINED = "aperture-redefined",
        APERTURE_NOT_SELECTED = "aperture-not-selected",
        APERTURE_UNDEFINED = "aperture-undefined",
        STEP_REPEAT_INVALID = "step-repeat-invalid",
        BLOCK_EMPTY = "block-empty",
        D_CODE_INVALID = "d-code-invalid",
        IMPLIED_OPERATION = "implied-operation",
        OPERATION_MISSING = "operation-missing",
        COORDINATE_MISSING = "coordinate-missing",
        START_POINT_MISSING = "start-point-missing",
        // Excellon
        HEADER_MISSING = "header-missing",
        FORMAT_COMMENT_INVALID = "format-comment-invalid",
        OPTION_UNKNOWN = "option-unknown",
        TOOL_NUMBER_MISSING = "tool-number-missing",
        TOOL_DIAMETER_INVALID = "tool-diameter-invalid",
        TOOL_DIAMETER_MISSING = "tool-diameter-missing",
        TOOL_PARAMETER_UNKNOWN = "tool-parameter-unknown",
        TOOL_REDEFINED = "tool-redefined",
        TOOL_UNDEFINED = "tool-undefined",
        TOOL_SELECTION_INVALID = "tool-selection-invalid",
        TOOL_NOT_SELECTED = "tool-not-selected",
        ARC_UNSUPPORTED = "arc-unsupported",
        // G-code
        LINE_TOO_LONG = "line-too-long",
        BLOCK_DELETE_READ = "block-delete-read",
        SYSTEM_COMMAND_IGNORED = "system-command-ignored",
        CHECKSUM_MISMATCH = "checksum-mismatch",
        CHECKSUM_INVALID = "checksum-invalid",
        COMMENT_UNCLOSED = "comment-unclosed",
        WORD_NUMBER_INVALID = "word-number-invalid",
        CHARACTER_IGNORED = "character-ignored",
        PLANE_UNSUPPORTED = "plane-unsupported",
        CUTTER_COMPENSATION_UNSUPPORTED = "cutter-compensation-unsupported",
        CANNED_CYCLE_UNSUPPORTED = "canned-cycle-unsupported",
        NEGATIVE_FEED = "negative-feed",
        NEGATIVE_SPINDLE = "negative-spindle",
        TOOL_NUMBER_INVALID = "tool-number-invalid",
        IMPLIED_RAPID = "implied-rapid",
        ARC_RADIUS_UNREACHABLE = "arc-radius-unreachable",
        ARC_CENTRE_MISSING = "arc-centre-missing",
        ARC_RADIUS_MISMATCH = "arc-radius-mismatch",
        PROGRAM_TOO_LONG = "program-too-long",
        UNITS_SUSPECT = "units-suspect",
        FEED_MISSING = "feed-missing",
    }
}

/// The units of the coordinates and lengths.
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

/// The units a Gerber or Excellon file is read in when it does not declare any.
pub const ASSUMED_UNITS: Units = Units::Millimetres;

/// The one rule for a file that does not declare its units.
///
/// A declared unit is returned as it is. When there is none, [`ASSUMED_UNITS`] (millimetres) is
/// returned and a warning with the code [`codes::UNITS_MISSING`] is added to `diagnostics` on
/// `line`. Excellon reads the digits of such a file with the metric default format (3.3).
pub fn resolve_units(
    declared: Option<Units>,
    line: usize,
    diagnostics: &mut Vec<Diagnostic>,
) -> Units {
    match declared {
        Some(units) => units,
        None => {
            diagnostics.push(Diagnostic::new(
                line,
                Severity::Warning,
                codes::UNITS_MISSING,
                "the file does not declare its units: millimetres are assumed",
            ));
            ASSUMED_UNITS
        }
    }
}

/// Which zeros an integer coordinate leaves out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroSuppression {
    /// Leading zeros are left out (`TZ` in Excellon).
    Leading,
    /// Trailing zeros are left out (`LZ` in Excellon).
    Trailing,
}

/// The largest magnitude of a number the readers accept; anything above is refused.
pub const MAX_NUMBER_MAGNITUDE: f64 = 1e15;

/// Why a number was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberError {
    /// The text is not a finite number.
    Invalid,
    /// The number is finite but its magnitude is above [`MAX_NUMBER_MAGNITUDE`].
    TooLarge,
}

/// Reads a decimal number, telling an invalid text from one that is too large.
pub fn parse_number(text: &str) -> Result<f64, NumberError> {
    let value: f64 = text.trim().parse().map_err(|_| NumberError::Invalid)?;
    if value.is_nan() {
        Err(NumberError::Invalid)
    } else if value.is_infinite() || value.abs() > MAX_NUMBER_MAGNITUDE {
        Err(NumberError::TooLarge)
    } else {
        Ok(value)
    }
}

/// Reads a decimal number; `None` when it is not a number or is above [`MAX_NUMBER_MAGNITUDE`].
pub fn parse_f64(text: &str) -> Option<f64> {
    parse_number(text).ok()
}

/// Converts the digits of a coordinate to a number.
///
/// A value with a decimal point is read as it is. Otherwise the digits are an integer: with
/// leading zeros suppressed it is divided by 10^decimals; with trailing zeros suppressed the digits
/// are first padded on the right to the full width of the format. More digits than the format
/// has are refused.
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
    // "-0" is plain zero, not negative zero
    Some(if negative && magnitude != 0.0 {
        -magnitude
    } else {
        magnitude
    })
}

/// The code of the diagnostic for a coordinate that [`decode_coordinate`] refused: too many digits
/// for the format is [`codes::NUMBER_TOO_LARGE`], anything else [`codes::COORDINATE_INVALID`].
pub fn coordinate_error_code(text: &str, integer_digits: u32, decimal_digits: u32) -> &'static str {
    let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
    let width = (integer_digits + decimal_digits) as usize;
    let all_digits = !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit());
    if (all_digits && digits.len() > width) || parse_number(digits) == Err(NumberError::TooLarge) {
        codes::NUMBER_TOO_LARGE
    } else {
        codes::COORDINATE_INVALID
    }
}

/// The size above which [`decode_input`] refuses a file by default: 256 MiB.
pub const DEFAULT_MAX_INPUT_BYTES: usize = 256 * 1024 * 1024;

/// Limits for reading bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputLimits {
    /// The largest input accepted, in bytes.
    pub max_bytes: usize,
}

impl Default for InputLimits {
    fn default() -> Self {
        InputLimits {
            max_bytes: DEFAULT_MAX_INPUT_BYTES,
        }
    }
}

/// Why bytes were not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputError {
    /// The input is larger than the limit.
    TooLarge {
        /// The size of the input.
        size: usize,
        /// The limit that applied.
        limit: usize,
    },
}

impl std::fmt::Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputError::TooLarge { size, limit } => {
                write!(f, "the input has {size} bytes, above the limit of {limit}")
            }
        }
    }
}

impl std::error::Error for InputError {}

/// Why a file was not read by [`read_file_limited`].
#[derive(Debug)]
pub enum ReadError {
    /// The file is larger than the limit; it was not read.
    TooLarge(InputError),
    /// The file could not be opened or read.
    Io(std::io::Error),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::TooLarge(e) => e.fmt(f),
            ReadError::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads a file, refusing it when it is larger than `limits.max_bytes` **before** it is read
/// into memory (by its size on disk, and by reading at most one byte more than the limit in
/// case the file grows or its size is not reported).
pub fn read_file_limited(
    path: &std::path::Path,
    limits: &InputLimits,
) -> Result<Vec<u8>, ReadError> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(ReadError::Io)?;
    let size = file.metadata().map_err(ReadError::Io)?.len();
    if size > limits.max_bytes as u64 {
        return Err(ReadError::TooLarge(InputError::TooLarge {
            size: usize::try_from(size).unwrap_or(usize::MAX),
            limit: limits.max_bytes,
        }));
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.take((limits.max_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(ReadError::Io)?;
    if bytes.len() > limits.max_bytes {
        return Err(ReadError::TooLarge(InputError::TooLarge {
            size: bytes.len(),
            limit: limits.max_bytes,
        }));
    }
    Ok(bytes)
}

/// Bytes turned into text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInput {
    /// The text, without a UTF-8 byte order mark.
    pub text: String,
    /// A notice about the encoding, to put at the front of the reader's diagnostics (empty when
    /// the text is UTF-8).
    pub diagnostics: Vec<Diagnostic>,
}

/// Turns the bytes of a file into text.
///
/// UTF-8 (with or without a byte order mark) is taken as it is. Anything else is read as Latin-1
/// (every byte is the character with the same number) and a warning with the code
/// [`codes::INPUT_NOT_UTF8`] is returned: unlike a lossy UTF-8 conversion, no character is
/// replaced by U+FFFD. An input above `limits.max_bytes` is refused before it is looked at.
pub fn decode_input(bytes: &[u8], limits: &InputLimits) -> Result<DecodedInput, InputError> {
    if bytes.len() > limits.max_bytes {
        return Err(InputError::TooLarge {
            size: bytes.len(),
            limit: limits.max_bytes,
        });
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => Ok(DecodedInput {
            text: text.to_owned(),
            diagnostics: Vec::new(),
        }),
        Err(_) => Ok(DecodedInput {
            text: bytes.iter().map(|&b| char::from(b)).collect(),
            diagnostics: vec![Diagnostic::new(
                1,
                Severity::Warning,
                codes::INPUT_NOT_UTF8,
                "the text is not UTF-8 and was read as Latin-1",
            )],
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minus_zero_decodes_to_plain_zero() {
        for text in ["-0", "-000", "-0.0", "-.0"] {
            let v = decode_coordinate(text, 2, 4, ZeroSuppression::Leading).unwrap();
            assert_eq!(v, 0.0, "{text}");
            assert!(v.is_sign_positive(), "{text} gave negative zero");
        }
        assert_eq!(
            decode_coordinate("-5000", 2, 4, ZeroSuppression::Leading),
            Some(-0.5)
        );
    }

    #[test]
    fn codes_are_unique_and_kebab_case() {
        let mut seen = std::collections::BTreeSet::new();
        for code in codes::ALL {
            assert!(seen.insert(*code), "{code} is defined twice");
            assert!(
                code.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{code}"
            );
        }
    }

    #[test]
    fn docs_list_every_code() {
        let docs = include_str!("../../../docs/DIAGNOSTICS.md");
        let listed: Vec<&str> = docs
            .lines()
            .filter_map(|l| l.strip_prefix("| `"))
            .filter_map(|l| l.split('`').next())
            .collect();
        for code in codes::ALL {
            assert!(
                listed.contains(code),
                "{code} is not in docs/DIAGNOSTICS.md"
            );
        }
        for code in &listed {
            assert!(
                codes::ALL.contains(code),
                "{code} is documented but not defined"
            );
        }
    }

    #[test]
    fn numbers_have_limits() {
        assert_eq!(parse_number("12.5"), Ok(12.5));
        assert_eq!(parse_number("abc"), Err(NumberError::Invalid));
        assert_eq!(parse_number("NaN"), Err(NumberError::Invalid));
        assert_eq!(parse_number("1e999"), Err(NumberError::TooLarge));
        assert_eq!(parse_number("1e16"), Err(NumberError::TooLarge));
        assert_eq!(parse_f64("1e15"), Some(1e15));
        assert_eq!(parse_f64("-2e15"), None);
    }

    #[test]
    fn coordinates_follow_the_format() {
        let lead = ZeroSuppression::Leading;
        assert_eq!(decode_coordinate("999999", 2, 4, lead), Some(99.9999));
        assert_eq!(decode_coordinate("1000000", 2, 4, lead), None);
        assert_eq!(decode_coordinate("-5000", 2, 4, lead), Some(-0.5));
        assert_eq!(
            decode_coordinate("12", 2, 4, ZeroSuppression::Trailing),
            Some(12.0)
        );
        assert_eq!(
            coordinate_error_code("1000000", 2, 4),
            codes::NUMBER_TOO_LARGE
        );
        assert_eq!(coordinate_error_code("1a", 2, 4), codes::COORDINATE_INVALID);
        assert_eq!(coordinate_error_code("", 2, 4), codes::COORDINATE_INVALID);
    }

    #[test]
    fn units_missing_has_one_rule() {
        let mut list = Vec::new();
        assert_eq!(
            resolve_units(Some(Units::Inches), 1, &mut list),
            Units::Inches
        );
        assert!(list.is_empty());
        assert_eq!(resolve_units(None, 3, &mut list), ASSUMED_UNITS);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].code, codes::UNITS_MISSING);
        assert_eq!(list[0].severity, Severity::Warning);
        assert_eq!(list[0].line, 3);
    }

    #[test]
    fn input_is_limited_and_decoded() {
        let limits = InputLimits { max_bytes: 8 };
        assert_eq!(
            decode_input(b"123456789", &limits),
            Err(InputError::TooLarge { size: 9, limit: 8 })
        );
        let ok = decode_input(b"M48\n", &InputLimits::default()).unwrap();
        assert_eq!(ok.text, "M48\n");
        assert!(ok.diagnostics.is_empty());
        let bom = decode_input(b"\xEF\xBB\xBFM48", &InputLimits::default()).unwrap();
        assert_eq!(bom.text, "M48");
        assert!(bom.diagnostics.is_empty());
        let latin = decode_input(b"; caf\xE9\n", &InputLimits::default()).unwrap();
        assert_eq!(latin.text, "; café\n");
        assert_eq!(latin.diagnostics[0].code, codes::INPUT_NOT_UTF8);
    }
}
