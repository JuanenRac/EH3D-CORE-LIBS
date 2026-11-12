//! The size limits of the text the readers take, kept in `input_limits.cfg` (the file that comes with this crate) and not in the code: how big a text may be before it is
//! refused and from where it is read with a warning. [`crate::InputLimits`] keeps the limit of the bytes for the functions that read bytes; this adds the lines and the warning.

use std::fmt;

use crate::{InputError, InputLimits};

/// The shipped file with the values.
pub const DEFAULT_CONFIG: &str = include_str!("../input_limits.cfg");

/// The limits of the size of a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLimits {
    /// The most bytes and a warning from where the text is big.
    pub input: InputLimits,
    pub warn_bytes: usize,
    /// The most lines and a warning from where the text is big.
    pub max_lines: usize,
    pub warn_lines: usize,
}

/// What the check of a text found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeVerdict {
    /// Inside the limits.
    Ok,
    /// Readable, but big: a program should say so.
    Warn,
}

/// Why a text is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextLimitError {
    /// More bytes than the limit.
    Bytes(InputError),
    /// More lines than the limit.
    TooManyLines { lines: usize, limit: usize },
}

impl fmt::Display for TextLimitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TextLimitError::Bytes(e) => fmt::Display::fmt(e, f),
            TextLimitError::TooManyLines { lines, limit } => {
                write!(f, "the input has {lines} lines, above the limit of {limit}")
            }
        }
    }
}

impl std::error::Error for TextLimitError {}

/// A line of the configuration that cannot be used (`line` is 0 for a fault of the whole file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    pub line: usize,
    pub text: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.text)
    }
}

impl std::error::Error for ConfigError {}

const KEYS: [&str; 4] = ["max_bytes", "max_lines", "warn_bytes", "warn_lines"];

impl Default for TextLimits {
    fn default() -> TextLimits {
        TextLimits::from_config(DEFAULT_CONFIG).expect("the shipped input_limits.cfg is valid")
    }
}

impl TextLimits {
    /// Reads the limits from the text of a configuration file: every key once, a whole number above zero, and a warning limit not above its hard limit.
    pub fn from_config(text: &str) -> Result<TextLimits, ConfigError> {
        let mut found: [Option<usize>; 4] = [None; 4];
        for (n, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let bad = |why: &str| ConfigError {
                line: n + 1,
                text: why.to_string(),
            };
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| bad("expected key = value"))?;
            let slot = KEYS
                .iter()
                .position(|k| *k == key.trim())
                .ok_or_else(|| bad("unknown key"))?;
            if found[slot].is_some() {
                return Err(bad("key given twice"));
            }
            let v: usize = value
                .trim()
                .parse()
                .map_err(|_| bad("not a whole number"))?;
            if v == 0 {
                return Err(bad("must be above zero"));
            }
            found[slot] = Some(v);
        }
        let get = |i: usize| {
            found[i].ok_or_else(|| ConfigError {
                line: 0,
                text: format!("missing key {}", KEYS[i]),
            })
        };
        let limits = TextLimits {
            input: InputLimits { max_bytes: get(0)? },
            max_lines: get(1)?,
            warn_bytes: get(2)?,
            warn_lines: get(3)?,
        };
        if limits.warn_bytes > limits.input.max_bytes || limits.warn_lines > limits.max_lines {
            return Err(ConfigError {
                line: 0,
                text: "a warning limit is above its hard limit".into(),
            });
        }
        Ok(limits)
    }

    /// Checks a text before it is read: refused past a hard limit, `Warn` past a warning limit.
    pub fn check(&self, text: &str) -> Result<SizeVerdict, TextLimitError> {
        let bytes = text.len();
        if bytes > self.input.max_bytes {
            return Err(TextLimitError::Bytes(InputError::TooLarge {
                size: bytes,
                limit: self.input.max_bytes,
            }));
        }
        let lines = text.bytes().filter(|&b| b == b'\n').count() + 1;
        if lines > self.max_lines {
            return Err(TextLimitError::TooManyLines {
                lines,
                limit: self.max_lines,
            });
        }
        if bytes > self.warn_bytes || lines > self.warn_lines {
            return Ok(SizeVerdict::Warn);
        }
        Ok(SizeVerdict::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = "max_bytes = 100\nmax_lines = 5\nwarn_bytes = 50\nwarn_lines = 3\n";

    #[test]
    fn the_shipped_file_is_valid_and_keeps_the_limit_of_the_bytes_of_the_readers() {
        let l = TextLimits::default();
        assert_eq!(l.input, InputLimits::default());
        assert!(l.warn_bytes < l.input.max_bytes && l.warn_lines < l.max_lines);
    }

    #[test]
    fn a_text_is_ok_warned_or_refused_by_its_bytes_and_its_lines() {
        let l = TextLimits::from_config(SMALL).unwrap();
        assert_eq!(l.check("a\nb\n"), Ok(SizeVerdict::Ok));
        assert_eq!(l.check("a\nb\nc\nd\n"), Ok(SizeVerdict::Warn));
        assert_eq!(l.check(&"x".repeat(60)), Ok(SizeVerdict::Warn));
        assert!(matches!(
            l.check(&"x".repeat(101)),
            Err(TextLimitError::Bytes(_))
        ));
        let e = l.check("1\n2\n3\n4\n5\n6\n").unwrap_err();
        assert_eq!(e, TextLimitError::TooManyLines { lines: 7, limit: 5 });
        assert!(e.to_string().contains("7 lines"));
    }

    #[test]
    fn a_bad_configuration_is_refused_with_its_line() {
        for (text, line) in [
            ("max_bytes 5\n", 1),
            ("max_bytes = x\n", 1),
            ("max_bytes = 0\n", 1),
            ("size = 5\n", 1),
            ("max_bytes = 5\nmax_bytes = 6\n", 2),
            ("max_bytes = 5\n", 0),
            (
                "max_bytes = 5\nmax_lines = 5\nwarn_bytes = 6\nwarn_lines = 1\n",
                0,
            ),
        ] {
            assert_eq!(
                TextLimits::from_config(text).unwrap_err().line,
                line,
                "{text}"
            );
        }
    }
}
