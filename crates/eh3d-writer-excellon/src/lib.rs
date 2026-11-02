//! # eh3d-writer-excellon
//!
//! A deterministic writer for Excellon (NC drill) files. It writes the model that
//! [`eh3d_parser_excellon::parse`] reads: tools, holes, slots (`G85`) and simple routes
//! (`M15` / `G01` / `M16`).
//!
//! Determinism: the same [`ExcellonFile`] and the same [`ExcellonOptions`] always give the same
//! bytes. The writer never reads the clock and never adds a date, a host or a version; the only
//! comments are the ones the caller gives. Lines end with a single line feed.
//!
//! Order: tools are written in number order and, for each tool, its holes, then its slots, then its
//! routes, each in the order of the model. Reading the file back gives the same holes, slots and
//! routes per tool; the order between different tools is the order of the tool numbers.
//!
//! Coordinates: by default with a decimal point (six decimals), which needs no zero suppression to
//! be read correctly. The fixed style writes integers of a stated format with leading or trailing
//! zeros suppressed. A value that does not fit is an error, never a wrong file.
//!
//! Not done: incremental coordinates, arcs in routes and the tool-change codes of drilling machines
//! beyond `T`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use eh3d_parser_excellon::{ExcellonFile, Point, Units, ZeroSuppression};

/// The units used when neither the options nor the file give any.
pub const DEFAULT_UNITS: Units = Units::Millimetres;
/// The zero suppression used when neither the options nor the file give any.
pub const DEFAULT_ZEROS: ZeroSuppression = ZeroSuppression::Leading;
/// Decimals of the default coordinate style.
pub const DEFAULT_DECIMALS: u32 = 6;
/// The most decimals any style writes.
const MAX_DECIMALS: u32 = 8;

/// How coordinates are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateStyle {
    /// A number with a decimal point and this many decimals (at least one is always written).
    Decimal {
        /// Decimals, 1 to 8.
        decimals: u32,
    },
    /// An integer of a fixed format, with the zeros of the file's suppression left out. The format
    /// is announced in a `FILE_FORMAT=i:d` comment.
    Fixed {
        /// Digits before the point, 1 to 8.
        integer_digits: u32,
        /// Digits after the point, 1 to 8.
        decimal_digits: u32,
    },
}

/// Why a file could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    /// The style asks for a number of digits outside 1 to 8.
    InvalidStyle,
    /// A number is NaN or infinite.
    NotFinite,
    /// A coordinate has more digits than the fixed format holds.
    CoordinateOverflow,
    /// A tool number outside 1 to 99, or a tool without a positive diameter.
    BadTool(u32),
    /// A hole, slot or route uses a tool that the file does not define.
    UndefinedTool(u32),
    /// A route without a tool or with fewer than two points.
    BadRoute,
    /// A comment holds a line break.
    ForbiddenCharacter,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            WriteError::InvalidStyle => "the coordinate style is not usable".to_owned(),
            WriteError::NotFinite => "a number is not finite".to_owned(),
            WriteError::CoordinateOverflow => {
                "a coordinate does not fit the fixed format".to_owned()
            }
            WriteError::BadTool(n) => {
                format!("tool {n} has a number outside 1 to 99 or no positive diameter")
            }
            WriteError::UndefinedTool(n) => format!("tool {n} is used but not defined"),
            WriteError::BadRoute => "a route needs a tool and at least two points".to_owned(),
            WriteError::ForbiddenCharacter => "a comment holds a line break".to_owned(),
        };
        f.write_str(&text)
    }
}

impl std::error::Error for WriteError {}

/// What the caller chooses.
#[derive(Debug, Clone, PartialEq)]
pub struct ExcellonOptions {
    /// The coordinate style.
    pub style: CoordinateStyle,
    /// The units. When `None`, the units of the file, then [`DEFAULT_UNITS`].
    pub units: Option<Units>,
    /// The zero suppression of the fixed style. When `None`, the one of the file, then
    /// [`DEFAULT_ZEROS`]. The decimal style ignores it (it is still announced in the header).
    pub zeros: Option<ZeroSuppression>,
    /// Comment lines (`;`) written after `M48`. The writer adds none of its own.
    pub comments: Vec<String>,
}

impl Default for ExcellonOptions {
    fn default() -> Self {
        ExcellonOptions {
            style: CoordinateStyle::Decimal {
                decimals: DEFAULT_DECIMALS,
            },
            units: None,
            zeros: None,
            comments: Vec::new(),
        }
    }
}

/// Rounds half to even.
fn round_half_even(x: f64) -> f64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 {
        2.0 * (x / 2.0).round()
    } else {
        r
    }
}

fn digits_ok(n: u32) -> bool {
    (1..=MAX_DECIMALS).contains(&n)
}

/// A number with a decimal point and exactly `decimals` decimals, trailing zeros trimmed down to
/// one decimal.
fn decimal(value: f64, decimals: u32) -> Result<String, WriteError> {
    if !value.is_finite() {
        return Err(WriteError::NotFinite);
    }
    let scale = 10f64.powi(decimals as i32);
    let rounded = round_half_even(value * scale) / scale;
    let mut s = format!("{:.*}", decimals as usize, rounded);
    while s.ends_with('0') && !s.ends_with(".0") {
        s.pop();
    }
    if s == "-0.0" {
        s = "0.0".to_owned();
    }
    Ok(s)
}

/// A plain number for tool sizes: up to six decimals, no trailing zeros, no exponent.
fn plain(value: f64) -> Result<String, WriteError> {
    let mut s = decimal(value, 6)?;
    if s.ends_with(".0") {
        s.truncate(s.len() - 2);
    }
    Ok(s)
}

/// An integer of the fixed format with the chosen zeros left out.
fn fixed(
    value: f64,
    integer: u32,
    decimals: u32,
    zeros: ZeroSuppression,
) -> Result<String, WriteError> {
    if !value.is_finite() {
        return Err(WriteError::NotFinite);
    }
    let scaled = round_half_even(value * 10f64.powi(decimals as i32));
    if scaled.abs() >= 10f64.powi((integer + decimals) as i32) {
        return Err(WriteError::CoordinateOverflow);
    }
    let magnitude = scaled.abs() as u64;
    let sign = if magnitude != 0 && scaled < 0.0 {
        "-"
    } else {
        ""
    };
    let width = (integer + decimals) as usize;
    let digits = match zeros {
        ZeroSuppression::Leading => magnitude.to_string(),
        ZeroSuppression::Trailing => {
            let padded = format!("{magnitude:0>width$}");
            let trimmed = padded.trim_end_matches('0');
            if trimmed.is_empty() {
                "0".to_owned()
            } else {
                trimmed.to_owned()
            }
        }
    };
    Ok(format!("{sign}{digits}"))
}

struct Writer {
    style: CoordinateStyle,
    zeros: ZeroSuppression,
}

impl Writer {
    fn value(&self, v: f64) -> Result<String, WriteError> {
        match self.style {
            CoordinateStyle::Decimal { decimals } => decimal(v, decimals),
            CoordinateStyle::Fixed {
                integer_digits,
                decimal_digits,
            } => fixed(v, integer_digits, decimal_digits, self.zeros),
        }
    }

    fn xy(&self, p: Point) -> Result<String, WriteError> {
        Ok(format!("X{}Y{}", self.value(p.x)?, self.value(p.y)?))
    }
}

/// Writes `file` as an Excellon drill file.
pub fn write(file: &ExcellonFile, options: &ExcellonOptions) -> Result<String, WriteError> {
    match options.style {
        CoordinateStyle::Decimal { decimals } if !digits_ok(decimals) => {
            return Err(WriteError::InvalidStyle)
        }
        CoordinateStyle::Fixed {
            integer_digits,
            decimal_digits,
        } if !digits_ok(integer_digits) || !digits_ok(decimal_digits) => {
            return Err(WriteError::InvalidStyle)
        }
        _ => {}
    }
    let units = options.units.or(file.units).unwrap_or(DEFAULT_UNITS);
    let zeros = options.zeros.or(file.zeros).unwrap_or(DEFAULT_ZEROS);
    let w = Writer {
        style: options.style,
        zeros,
    };

    let mut out = String::from("M48\n");
    for comment in &options.comments {
        if comment.contains('\n') || comment.contains('\r') {
            return Err(WriteError::ForbiddenCharacter);
        }
        out.push_str(&format!(";{comment}\n"));
    }
    if let CoordinateStyle::Fixed {
        integer_digits,
        decimal_digits,
    } = options.style
    {
        out.push_str(&format!(";FILE_FORMAT={integer_digits}:{decimal_digits}\n"));
    }
    out.push_str(match units {
        Units::Millimetres => "METRIC",
        Units::Inches => "INCH",
    });
    out.push_str(match zeros {
        ZeroSuppression::Leading => ",TZ\n",
        ZeroSuppression::Trailing => ",LZ\n",
    });
    for (number, tool) in &file.tools {
        if !(1..=99).contains(number) || !(tool.diameter.is_finite() && tool.diameter > 0.0) {
            return Err(WriteError::BadTool(*number));
        }
        out.push_str(&format!("T{number:02}C{}", plain(tool.diameter)?));
        if let Some(feed) = tool.feed {
            out.push_str(&format!("F{}", plain(feed)?));
        }
        if let Some(speed) = tool.speed {
            out.push_str(&format!("S{}", plain(speed)?));
        }
        out.push('\n');
    }
    out.push_str("%\n");

    let mut used: Vec<u32> = file
        .drills
        .iter()
        .map(|h| h.tool)
        .chain(file.slots.iter().map(|s| s.tool))
        .chain(file.routes.iter().map(|r| r.tool.unwrap_or(0)))
        .collect();
    used.sort_unstable();
    used.dedup();
    for tool in used {
        if tool == 0 {
            return Err(WriteError::BadRoute);
        }
        if !file.tools.contains_key(&tool) {
            return Err(WriteError::UndefinedTool(tool));
        }
        out.push_str(&format!("T{tool:02}\n"));
        for hit in file.drills.iter().filter(|h| h.tool == tool) {
            out.push_str(&w.xy(hit.at)?);
            out.push('\n');
        }
        for slot in file.slots.iter().filter(|s| s.tool == tool) {
            out.push_str(&format!("{}G85{}\n", w.xy(slot.from)?, w.xy(slot.to)?));
        }
        for route in file.routes.iter().filter(|r| r.tool == Some(tool)) {
            let (first, rest) = route.points.split_first().ok_or(WriteError::BadRoute)?;
            if rest.is_empty() {
                return Err(WriteError::BadRoute);
            }
            out.push_str(&format!("G00{}\nM15\n", w.xy(*first)?));
            for point in rest {
                out.push_str(&format!("G01{}\n", w.xy(*point)?));
            }
            out.push_str("M16\n");
        }
    }
    out.push_str("M30\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_parser_excellon::{parse, Hit, Route, Slot, Tool};

    const SAMPLE: &str = "M48\nMETRIC,TZ\nT01C0.8F200S65\nT02C1.2\nT03C2\n%\nT01\nX1.5Y2.25\nX10.0Y-3.125\nX0.0Y0.0\nT02\nX5.0Y5.0\nX12.0Y6.0G85X14.0Y6.0\nT03\nG00X20.0Y20.0\nM15\nG01X30.0Y20.0\nG01X30.0Y30.0\nM16\nM30\n";

    fn write_ok(file: &ExcellonFile, options: &ExcellonOptions) -> String {
        write(file, options).unwrap()
    }

    type Stripped = (
        Vec<(u32, Point)>,
        Vec<(u32, Point, Point)>,
        Vec<(Option<u32>, Vec<Point>)>,
    );

    fn strip(file: &ExcellonFile) -> Stripped {
        (
            file.drills.iter().map(|h| (h.tool, h.at)).collect(),
            file.slots.iter().map(|s| (s.tool, s.from, s.to)).collect(),
            file.routes
                .iter()
                .map(|r| (r.tool, r.points.clone()))
                .collect(),
        )
    }

    #[test]
    fn the_sample_reads_clean() {
        let file = parse(SAMPLE);
        assert!(file.is_clean(), "{:?}", file.diagnostics);
        assert_eq!(file.tools.len(), 3);
        assert_eq!(file.drills.len(), 4);
        assert_eq!(file.slots.len(), 1);
        assert_eq!(file.routes.len(), 1);
    }

    #[test]
    fn write_then_read_gives_the_same_model_in_every_style() {
        let file = parse(SAMPLE);
        let styles = [
            CoordinateStyle::Decimal { decimals: 6 },
            CoordinateStyle::Decimal { decimals: 3 },
            CoordinateStyle::Fixed {
                integer_digits: 3,
                decimal_digits: 3,
            },
            CoordinateStyle::Fixed {
                integer_digits: 4,
                decimal_digits: 6,
            },
        ];
        for style in styles {
            for zeros in [ZeroSuppression::Leading, ZeroSuppression::Trailing] {
                let options = ExcellonOptions {
                    style,
                    zeros: Some(zeros),
                    ..ExcellonOptions::default()
                };
                let text = write_ok(&file, &options);
                let again = parse(&text);
                assert!(
                    again.is_clean(),
                    "{style:?} {zeros:?}: {:?}\n{text}",
                    again.diagnostics
                );
                assert_eq!(again.tools, file.tools, "{style:?} {zeros:?}\n{text}");
                assert_eq!(again.units, file.units);
                assert_eq!(strip(&again), strip(&file), "{style:?} {zeros:?}\n{text}");
                assert!(again.ended);
            }
        }
    }

    #[test]
    fn the_second_round_is_byte_identical() {
        let text = write_ok(&parse(SAMPLE), &ExcellonOptions::default());
        assert_eq!(write_ok(&parse(&text), &ExcellonOptions::default()), text);
        assert_eq!(text, SAMPLE);
    }

    #[test]
    fn the_same_input_gives_the_same_bytes_every_time() {
        let file = parse(SAMPLE);
        let first = write_ok(&file, &ExcellonOptions::default());
        for _ in 0..5 {
            assert_eq!(write_ok(&file.clone(), &ExcellonOptions::default()), first);
        }
        for forbidden in ["DATE", "Date", "date", "VER", "GenerationSoftware", ";"] {
            assert!(!first.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn caller_comments_and_the_fixed_format_are_announced() {
        let file = parse(SAMPLE);
        let options = ExcellonOptions {
            style: CoordinateStyle::Fixed {
                integer_digits: 3,
                decimal_digits: 3,
            },
            comments: vec!["made by a test".into()],
            ..ExcellonOptions::default()
        };
        let text = write_ok(&file, &options);
        assert!(
            text.starts_with("M48\n;made by a test\n;FILE_FORMAT=3:3\nMETRIC,TZ\n"),
            "{text}"
        );
        assert!(text.contains("X1500Y2250\n"), "{text}");
        assert!(text.contains("X10000Y-3125\n"), "{text}");
        assert!(text.contains("X0Y0\n"), "{text}");
    }

    #[test]
    fn trailing_zero_suppression_pads_then_trims() {
        assert_eq!(fixed(1.5, 3, 3, ZeroSuppression::Trailing).unwrap(), "0015");
        assert_eq!(fixed(10.0, 3, 3, ZeroSuppression::Trailing).unwrap(), "01");
        assert_eq!(fixed(0.0, 3, 3, ZeroSuppression::Trailing).unwrap(), "0");
        assert_eq!(
            fixed(-3.125, 3, 3, ZeroSuppression::Trailing).unwrap(),
            "-003125"
        );
        assert_eq!(fixed(1.5, 3, 3, ZeroSuppression::Leading).unwrap(), "1500");
        assert_eq!(fixed(-0.0, 3, 3, ZeroSuppression::Leading).unwrap(), "0");
        assert_eq!(fixed(0.0004, 3, 3, ZeroSuppression::Leading).unwrap(), "0");
        assert_eq!(fixed(-0.0004, 3, 3, ZeroSuppression::Leading).unwrap(), "0");
        assert_eq!(
            fixed(1000.0, 3, 3, ZeroSuppression::Leading),
            Err(WriteError::CoordinateOverflow)
        );
        assert_eq!(
            fixed(f64::NAN, 3, 3, ZeroSuppression::Leading),
            Err(WriteError::NotFinite)
        );
    }

    #[test]
    fn decimals_always_keep_a_point_and_round_half_to_even() {
        assert_eq!(decimal(5.0, 6).unwrap(), "5.0");
        assert_eq!(decimal(-0.0, 6).unwrap(), "0.0");
        assert_eq!(decimal(0.25, 1).unwrap(), "0.2");
        assert_eq!(decimal(0.35, 1).unwrap(), "0.4");
        assert_eq!(decimal(1.0000004, 6).unwrap(), "1.0");
        assert_eq!(decimal(12.3456789, 6).unwrap(), "12.345679");
        assert_eq!(plain(0.8).unwrap(), "0.8");
        assert_eq!(plain(2.0).unwrap(), "2");
        assert_eq!(decimal(f64::INFINITY, 3), Err(WriteError::NotFinite));
    }

    #[test]
    fn inch_files_keep_their_units() {
        let file = parse("M48\nINCH,LZ\nT01C0.0315\n%\nT01\nX0.5Y0.25\nM30\n");
        assert!(file.is_clean(), "{:?}", file.diagnostics);
        let text = write_ok(&file, &ExcellonOptions::default());
        assert!(text.contains("INCH,LZ\n"), "{text}");
        let again = parse(&text);
        assert_eq!(strip(&again), strip(&file));
        assert_eq!(again.units, Some(Units::Inches));
        let metric = write_ok(
            &file,
            &ExcellonOptions {
                units: Some(Units::Millimetres),
                ..Default::default()
            },
        );
        assert!(metric.contains("METRIC,LZ\n"));
    }

    #[test]
    fn holes_of_different_tools_are_grouped_by_tool_number() {
        let mut file = parse(
            "M48\nMETRIC,TZ\nT01C1\nT02C2\n%\nT02\nX1.0Y1.0\nT01\nX2.0Y2.0\nT02\nX3.0Y3.0\nM30\n",
        );
        assert!(file.is_clean());
        let text = write_ok(&file, &ExcellonOptions::default());
        let body = text.split("%\n").nth(1).unwrap();
        assert_eq!(body, "T01\nX2.0Y2.0\nT02\nX1.0Y1.0\nX3.0Y3.0\nM30\n");
        file.drills.reverse();
        let reversed = write_ok(&file, &ExcellonOptions::default());
        assert!(
            reversed.contains("T02\nX3.0Y3.0\nX1.0Y1.0\n"),
            "within a tool the model order is kept"
        );
    }

    #[test]
    fn what_cannot_be_written_is_an_error() {
        let base = parse(SAMPLE);
        let no = |style| {
            write(
                &base,
                &ExcellonOptions {
                    style,
                    ..Default::default()
                },
            )
        };
        assert_eq!(
            no(CoordinateStyle::Decimal { decimals: 0 }),
            Err(WriteError::InvalidStyle)
        );
        assert_eq!(
            no(CoordinateStyle::Decimal { decimals: 9 }),
            Err(WriteError::InvalidStyle)
        );
        assert_eq!(
            no(CoordinateStyle::Fixed {
                integer_digits: 0,
                decimal_digits: 3
            }),
            Err(WriteError::InvalidStyle)
        );
        assert_eq!(
            no(CoordinateStyle::Fixed {
                integer_digits: 1,
                decimal_digits: 3
            }),
            Err(WriteError::CoordinateOverflow),
            "the sample has 20 mm coordinates"
        );
        assert_eq!(
            write(
                &base,
                &ExcellonOptions {
                    comments: vec!["a\nb".into()],
                    ..Default::default()
                }
            ),
            Err(WriteError::ForbiddenCharacter)
        );

        let mut file = base.clone();
        file.drills.push(Hit {
            line: 1,
            tool: 9,
            at: Point { x: 0.0, y: 0.0 },
        });
        assert_eq!(
            write(&file, &ExcellonOptions::default()),
            Err(WriteError::UndefinedTool(9))
        );

        let mut file = base.clone();
        file.tools.insert(
            100,
            Tool {
                number: 100,
                diameter: 1.0,
                feed: None,
                speed: None,
            },
        );
        assert_eq!(
            write(&file, &ExcellonOptions::default()),
            Err(WriteError::BadTool(100))
        );

        let mut file = base.clone();
        file.tools.insert(
            5,
            Tool {
                number: 5,
                diameter: 0.0,
                feed: None,
                speed: None,
            },
        );
        assert_eq!(
            write(&file, &ExcellonOptions::default()),
            Err(WriteError::BadTool(5))
        );

        let mut file = base.clone();
        file.routes.push(Route {
            line: 1,
            tool: Some(3),
            points: vec![Point { x: 1.0, y: 1.0 }],
        });
        assert_eq!(
            write(&file, &ExcellonOptions::default()),
            Err(WriteError::BadRoute)
        );

        let mut file = base.clone();
        file.routes.push(Route {
            line: 1,
            tool: None,
            points: vec![Point { x: 1.0, y: 1.0 }, Point { x: 2.0, y: 2.0 }],
        });
        assert_eq!(
            write(&file, &ExcellonOptions::default()),
            Err(WriteError::BadRoute)
        );

        let mut file = base;
        file.slots.push(Slot {
            line: 1,
            tool: 2,
            from: Point {
                x: f64::NAN,
                y: 0.0,
            },
            to: Point { x: 0.0, y: 0.0 },
        });
        assert_eq!(
            write(&file, &ExcellonOptions::default()),
            Err(WriteError::NotFinite)
        );
    }

    #[test]
    fn an_empty_file_is_a_valid_file() {
        let text = write_ok(&parse(""), &ExcellonOptions::default());
        assert_eq!(text, "M48\nMETRIC,TZ\n%\nM30\n");
        assert!(parse(&text).ended);
    }

    /// A small seeded generator (SplitMix64) so the generated files are the same in every run.
    struct Seeded(u64);
    impl Seeded {
        fn next(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
        fn coordinate(&mut self) -> f64 {
            // a multiple of one micrometre inside +-400 mm
            ((self.next() % 800_000_001) as i64 - 400_000_000) as f64 / 1_000_000.0
        }
    }

    #[test]
    fn generated_drill_files_survive_write_and_read_in_every_style() {
        let mut rng = Seeded(11);
        for round in 0..40 {
            let mut file = parse(
                "M48
METRIC,TZ
T01C0.8
T02C1
T05C3.175
%
M30
",
            );
            for i in 0..(rng.next() % 12 + 1) {
                let tool = [1, 2, 5][(rng.next() % 3) as usize];
                let at = Point {
                    x: rng.coordinate(),
                    y: rng.coordinate(),
                };
                if i % 4 == 3 {
                    let to = Point {
                        x: rng.coordinate(),
                        y: rng.coordinate(),
                    };
                    file.slots.push(Slot {
                        line: 0,
                        tool,
                        from: at,
                        to,
                    });
                } else {
                    file.drills.push(Hit { line: 0, tool, at });
                }
            }
            let style = if round % 2 == 0 {
                CoordinateStyle::Decimal { decimals: 6 }
            } else {
                CoordinateStyle::Fixed {
                    integer_digits: 3,
                    decimal_digits: 6,
                }
            };
            let zeros = if round % 4 < 2 {
                ZeroSuppression::Leading
            } else {
                ZeroSuppression::Trailing
            };
            let options = ExcellonOptions {
                style,
                zeros: Some(zeros),
                ..ExcellonOptions::default()
            };
            let text = write_ok(&file, &options);
            let again = parse(&text);
            assert!(
                again.is_clean(),
                "{style:?} {zeros:?} {:?}
{text}",
                again.diagnostics
            );
            let by_tool = |f: &ExcellonFile| {
                let mut hits: Vec<(u32, Point)> = f.drills.iter().map(|h| (h.tool, h.at)).collect();
                hits.sort_by_key(|e| e.0);
                let mut slots: Vec<(u32, Point, Point)> =
                    f.slots.iter().map(|s| (s.tool, s.from, s.to)).collect();
                slots.sort_by_key(|e| e.0);
                (hits, slots)
            };
            assert_eq!(
                by_tool(&again),
                by_tool(&file),
                "{style:?} {zeros:?}
{text}"
            );
            assert_eq!(write_ok(&again, &options), text, "writing twice is stable");
        }
    }
}
