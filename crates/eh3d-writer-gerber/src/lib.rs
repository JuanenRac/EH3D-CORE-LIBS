//! # eh3d-writer-gerber
//!
//! A deterministic writer for Gerber X2 (RS-274X) files. It writes the model that
//! [`eh3d_parser_gerber::parse`] reads, so a file can be read, changed and written again.
//!
//! Determinism: the same [`GerberFile`] and the same [`GerberOptions`] always give the same bytes.
//! The writer never reads the clock and never adds a date, a host or a version: a file attribute such
//! as `.CreationDate` or `.GenerationSoftware` appears only when the caller puts it among the
//! attributes. Lines end with a single line feed.
//!
//! Numbers: a coordinate is rounded half to even to the decimals of the coordinate format and written
//! as an integer without leading zeros (the only form current Gerber allows); a number that does not
//! fit the format is an error, never a wrong file. Both `X` and `Y` are written in every operation.
//!
//! Not done: file attributes other than `TF` (the model does not say where the others apply, so they
//! are counted in [`GerberOutput::dropped_attributes`] and left out), trailing-zero omission
//! (deprecated), the interpretation of aperture macro bodies (they are copied as text) and block
//! apertures.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt::Write as _;

use eh3d_parser_gerber::{
    Aperture, CoordinateFormat, GerberFile, Operation, Point, Polarity, QuadrantMode, Units,
    ZeroOmission,
};

/// The coordinate format used when neither the options nor the file give one: 4 digits before and 6
/// after the point.
pub const DEFAULT_FORMAT: CoordinateFormat = CoordinateFormat {
    zero_omission: ZeroOmission::Leading,
    integer_digits: 4,
    decimal_digits: 6,
};
/// The units used when neither the options nor the file give any.
pub const DEFAULT_UNITS: Units = Units::Millimetres;
/// Decimals kept in aperture sizes and other plain numbers.
const NUMBER_DECIMALS: usize = 6;

/// Why a file could not be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteError {
    /// The format asks for trailing zeros to be omitted.
    UnsupportedZeroOmission,
    /// The format has no digits or more than Gerber allows (1 to 6 before and after the point).
    InvalidFormat,
    /// A number is NaN or infinite.
    NotFinite,
    /// A coordinate has more digits than the format holds.
    CoordinateOverflow,
    /// An operation or an aperture refers to an aperture number that is not defined, or the number
    /// is below 10.
    BadAperture(u32),
    /// An aperture uses a macro that is not defined.
    MissingMacro(String),
    /// A text (a comment, a macro line, an attribute) holds a `*` or `%`, which would end it.
    ForbiddenCharacter,
    /// A polygon aperture with a number of vertices outside 3 to 12.
    BadPolygon,
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            WriteError::UnsupportedZeroOmission => {
                "omitting trailing zeros is not supported".to_owned()
            }
            WriteError::InvalidFormat => "the coordinate format is not usable".to_owned(),
            WriteError::NotFinite => "a number is not finite".to_owned(),
            WriteError::CoordinateOverflow => {
                "a coordinate does not fit the coordinate format".to_owned()
            }
            WriteError::BadAperture(n) => {
                format!("aperture {n} is not defined or its number is below 10")
            }
            WriteError::MissingMacro(name) => format!("the macro '{name}' is not defined"),
            WriteError::ForbiddenCharacter => "a text holds '*' or '%'".to_owned(),
            WriteError::BadPolygon => "a polygon aperture needs 3 to 12 vertices".to_owned(),
        };
        f.write_str(&text)
    }
}

impl std::error::Error for WriteError {}

/// What the caller chooses.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GerberOptions {
    /// The coordinate format. When `None`, the format of the file, then [`DEFAULT_FORMAT`].
    pub format: Option<CoordinateFormat>,
    /// The units. When `None`, the units of the file, then [`DEFAULT_UNITS`].
    pub units: Option<Units>,
    /// Comment lines (`G04`) written first. The writer adds none of its own.
    pub comments: Vec<String>,
}

/// The written file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GerberOutput {
    /// The text of the file.
    pub text: String,
    /// How many attributes other than `TF` were left out.
    pub dropped_attributes: usize,
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

/// One coordinate as the integer of the format: no point, no leading zeros, a minus sign if negative.
fn coordinate(value: f64, format: CoordinateFormat) -> Result<String, WriteError> {
    if !value.is_finite() {
        return Err(WriteError::NotFinite);
    }
    let scaled = round_half_even(value * 10f64.powi(format.decimal_digits as i32));
    let limit = 10f64.powi((format.integer_digits + format.decimal_digits) as i32);
    if scaled.abs() >= limit {
        return Err(WriteError::CoordinateOverflow);
    }
    let n = scaled as i64;
    Ok(n.to_string())
}

/// A plain number with at most six decimals and no trailing zeros.
fn number(value: f64) -> Result<String, WriteError> {
    if !value.is_finite() {
        return Err(WriteError::NotFinite);
    }
    let mut s = format!("{:.*}", NUMBER_DECIMALS, value);
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".to_owned();
    }
    Ok(s)
}

fn check_text(text: &str) -> Result<(), WriteError> {
    if text.contains('*') || text.contains('%') || text.contains('\n') || text.contains('\r') {
        Err(WriteError::ForbiddenCharacter)
    } else {
        Ok(())
    }
}

fn with_hole(mut base: String, hole: Option<f64>) -> Result<String, WriteError> {
    if let Some(h) = hole {
        write!(base, "X{}", number(h)?).expect("writing to a String");
    }
    Ok(base)
}

fn aperture_text(n: u32, aperture: &Aperture, file: &GerberFile) -> Result<String, WriteError> {
    let body = match aperture {
        Aperture::Circle { diameter, hole } => {
            with_hole(format!("C,{}", number(*diameter)?), *hole)?
        }
        Aperture::Rectangle { x, y, hole } => {
            with_hole(format!("R,{}X{}", number(*x)?, number(*y)?), *hole)?
        }
        Aperture::Obround { x, y, hole } => {
            with_hole(format!("O,{}X{}", number(*x)?, number(*y)?), *hole)?
        }
        Aperture::Polygon {
            diameter,
            vertices,
            rotation,
            hole,
        } => {
            if !(3..=12).contains(vertices) {
                return Err(WriteError::BadPolygon);
            }
            with_hole(
                format!(
                    "P,{}X{}X{}",
                    number(*diameter)?,
                    vertices,
                    number(*rotation)?
                ),
                *hole,
            )?
        }
        Aperture::Macro { name, params } => {
            check_text(name)?;
            if !file.macros.contains_key(name) {
                return Err(WriteError::MissingMacro(name.clone()));
            }
            if params.is_empty() {
                name.clone()
            } else {
                let list: Result<Vec<String>, _> = params.iter().map(|p| number(*p)).collect();
                format!("{name},{}", list?.join("X"))
            }
        }
    };
    Ok(format!("%ADD{n}{body}*%\n"))
}

/// The state of the drawing while the operations are written.
struct Body<'a> {
    out: String,
    format: CoordinateFormat,
    file: &'a GerberFile,
    at: Option<(String, String)>,
    interpolation: Option<&'static str>,
    quadrant: Option<QuadrantMode>,
}

impl Body<'_> {
    fn xy(&self, p: Point) -> Result<(String, String), WriteError> {
        Ok((coordinate(p.x, self.format)?, coordinate(p.y, self.format)?))
    }

    fn word(&mut self, text: &str) {
        self.out.push_str(text);
        self.out.push_str("*\n");
    }

    /// Moves to `p` with `D02` unless the pen is already there.
    fn move_to(&mut self, p: Point) -> Result<(), WriteError> {
        let xy = self.xy(p)?;
        if self.at.as_ref() != Some(&xy) {
            self.word(&format!("X{}Y{}D02", xy.0, xy.1));
            self.at = Some(xy);
        }
        Ok(())
    }

    fn interpolation(&mut self, code: &'static str) {
        if self.interpolation != Some(code) {
            self.word(code);
            self.interpolation = Some(code);
        }
    }

    fn operation(&mut self, op: &Operation) -> Result<(), WriteError> {
        match op {
            Operation::SelectAperture(n) => {
                if *n < 10 || !self.file.apertures.contains_key(n) {
                    return Err(WriteError::BadAperture(*n));
                }
                self.word(&format!("D{n}"));
            }
            Operation::Move { to } => {
                let xy = self.xy(*to)?;
                self.word(&format!("X{}Y{}D02", xy.0, xy.1));
                self.at = Some(xy);
            }
            Operation::Flash { at } => {
                let xy = self.xy(*at)?;
                self.word(&format!("X{}Y{}D03", xy.0, xy.1));
                self.at = Some(xy);
            }
            Operation::Line { from, to } => {
                self.move_to(*from)?;
                self.interpolation("G01");
                let xy = self.xy(*to)?;
                self.word(&format!("X{}Y{}D01", xy.0, xy.1));
                self.at = Some(xy);
            }
            Operation::Arc {
                from,
                to,
                center_offset,
                clockwise,
                quadrant,
            } => {
                self.move_to(*from)?;
                if self.quadrant != Some(*quadrant) {
                    self.word(match quadrant {
                        QuadrantMode::Multi => "G75",
                        QuadrantMode::Single => "G74",
                    });
                    self.quadrant = Some(*quadrant);
                }
                self.interpolation(if *clockwise { "G02" } else { "G03" });
                let xy = self.xy(*to)?;
                let i = coordinate(center_offset.0, self.format)?;
                let j = coordinate(center_offset.1, self.format)?;
                self.word(&format!("X{}Y{}I{i}J{j}D01", xy.0, xy.1));
                self.at = Some(xy);
            }
            Operation::RegionStart => self.word("G36"),
            Operation::RegionEnd => self.word("G37"),
            Operation::Polarity(p) => self.out.push_str(match p {
                Polarity::Dark => "%LPD*%\n",
                Polarity::Clear => "%LPC*%\n",
            }),
            Operation::StepRepeat(Some(sr)) => {
                self.out.push_str(&format!(
                    "%SRX{}Y{}I{}J{}*%\n",
                    sr.x_repeats,
                    sr.y_repeats,
                    number(sr.x_step)?,
                    number(sr.y_step)?
                ));
            }
            Operation::StepRepeat(None) => self.out.push_str("%SR*%\n"),
        }
        Ok(())
    }
}

/// Writes `file` as Gerber X2.
pub fn write(file: &GerberFile, options: &GerberOptions) -> Result<GerberOutput, WriteError> {
    let format = options.format.or(file.format).unwrap_or(DEFAULT_FORMAT);
    if format.zero_omission != ZeroOmission::Leading {
        return Err(WriteError::UnsupportedZeroOmission);
    }
    if !(1..=6).contains(&format.integer_digits) || !(1..=6).contains(&format.decimal_digits) {
        return Err(WriteError::InvalidFormat);
    }
    let units = options.units.or(file.units).unwrap_or(DEFAULT_UNITS);

    let mut text = String::new();
    for comment in &options.comments {
        check_text(comment)?;
        text.push_str(&format!("G04 {comment}*\n"));
    }
    let mut dropped = 0;
    for attribute in &file.attributes {
        if attribute.kind != "TF" {
            dropped += 1;
            continue;
        }
        check_text(&attribute.name)?;
        let mut line = format!("%TF{}", attribute.name);
        for value in &attribute.values {
            check_text(value)?;
            line.push(',');
            line.push_str(value);
        }
        text.push_str(&line);
        text.push_str("*%\n");
    }
    text.push_str(&format!(
        "%FSLAX{i}{d}Y{i}{d}*%\n",
        i = format.integer_digits,
        d = format.decimal_digits
    ));
    text.push_str(match units {
        Units::Millimetres => "%MOMM*%\n",
        Units::Inches => "%MOIN*%\n",
    });
    for (name, definition) in &file.macros {
        check_text(name)?;
        text.push_str(&format!("%AM{name}*\n"));
        for line in &definition.body {
            check_text(line)?;
            text.push_str(&format!("{line}*\n"));
        }
        text.push_str("%\n");
    }
    for (n, aperture) in &file.apertures {
        if *n < 10 {
            return Err(WriteError::BadAperture(*n));
        }
        text.push_str(&aperture_text(*n, aperture, file)?);
    }

    let mut body = Body {
        out: String::new(),
        format,
        file,
        at: None,
        interpolation: None,
        quadrant: None,
    };
    for command in &file.commands {
        body.operation(&command.op)?;
    }
    text.push_str(&body.out);
    text.push_str("M02*\n");
    Ok(GerberOutput {
        text,
        dropped_attributes: dropped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_parser_gerber::{parse, Attribute, MacroDefinition};

    const SAMPLE: &str = "%TF.FileFunction,Copper,L1,Top*%
%FSLAX46Y46*%
%MOMM*%
%ADD10C,0.25*%
%ADD11R,1.5X0.8*%
%ADD12O,2X1X0.3*%
%ADD13P,1.2X6X15*%
%LPD*%
D10*
X1000000Y2000000D02*
G01*
X5500000Y2000000D01*
X5500000Y7250000D01*
D11*
X3000000Y3000000D03*
D12*
X-1500000Y-2500000D03*
G36*
X0Y0D02*
X1000000Y0D01*
X1000000Y1000000D01*
X0Y0D01*
G37*
D10*
X2000000Y1000000D02*
G75*
G03*
X1000000Y2000000I-1000000J0D01*
M02*
";

    fn ops(file: &GerberFile) -> Vec<Operation> {
        file.commands.iter().map(|c| c.op.clone()).collect()
    }

    fn write_ok(file: &GerberFile) -> String {
        write(file, &GerberOptions::default()).unwrap().text
    }

    #[test]
    fn the_sample_reads_clean() {
        let file = parse(SAMPLE);
        assert!(file.is_clean(), "{:?}", file.diagnostics);
        assert_eq!(file.apertures.len(), 4);
        assert!(file.commands.len() >= 12);
    }

    #[test]
    fn write_then_read_gives_the_same_model() {
        let file = parse(SAMPLE);
        let text = write_ok(&file);
        let again = parse(&text);
        assert!(again.is_clean(), "{:?}\n{text}", again.diagnostics);
        assert_eq!(again.units, file.units);
        assert_eq!(again.format, file.format);
        assert_eq!(again.apertures, file.apertures);
        assert_eq!(again.attributes, file.attributes);
        assert_eq!(ops(&again), ops(&file), "\n{text}");
        assert!(again.ended);
    }

    #[test]
    fn the_second_round_is_byte_identical() {
        let text = write_ok(&parse(SAMPLE));
        assert_eq!(write_ok(&parse(&text)), text);
        // The sample is already in the form the writer produces.
        assert_eq!(text, SAMPLE);
    }

    #[test]
    fn the_same_input_gives_the_same_bytes_every_time() {
        let file = parse(SAMPLE);
        let first = write_ok(&file);
        for _ in 0..5 {
            assert_eq!(write_ok(&file.clone()), first);
        }
    }

    #[test]
    fn nothing_that_varies_is_written_unless_the_caller_gives_it() {
        let text = write_ok(&parse(SAMPLE));
        for forbidden in ["CreationDate", "GenerationSoftware", "G04", "ProjectId"] {
            assert!(!text.contains(forbidden), "{forbidden}");
        }
        let mut file = parse(SAMPLE);
        file.attributes.push(Attribute {
            kind: "TF".into(),
            name: ".CreationDate".into(),
            values: vec!["2000-01-01T00:00:00+00:00".into()],
        });
        let out = write(
            &file,
            &GerberOptions {
                comments: vec!["made by a test".into()],
                ..GerberOptions::default()
            },
        )
        .unwrap();
        assert!(out
            .text
            .starts_with("G04 made by a test*\n%TF.FileFunction"));
        assert!(out
            .text
            .contains("%TF.CreationDate,2000-01-01T00:00:00+00:00*%\n"));
    }

    #[test]
    fn coordinates_round_half_to_even_and_never_write_a_point() {
        let f = CoordinateFormat {
            zero_omission: ZeroOmission::Leading,
            integer_digits: 2,
            decimal_digits: 1,
        };
        assert_eq!(coordinate(0.25, f).unwrap(), "2");
        assert_eq!(coordinate(0.35, f).unwrap(), "4");
        assert_eq!(coordinate(-0.25, f).unwrap(), "-2");
        assert_eq!(coordinate(-0.0, f).unwrap(), "0");
        assert_eq!(coordinate(0.0, f).unwrap(), "0");
        assert_eq!(coordinate(12.3, f).unwrap(), "123");
        assert_eq!(coordinate(99.9, f).unwrap(), "999");
        assert_eq!(coordinate(100.0, f), Err(WriteError::CoordinateOverflow));
        assert_eq!(coordinate(-100.0, f), Err(WriteError::CoordinateOverflow));
        assert_eq!(coordinate(f64::NAN, f), Err(WriteError::NotFinite));
        assert_eq!(coordinate(f64::INFINITY, f), Err(WriteError::NotFinite));
    }

    #[test]
    fn plain_numbers_have_no_exponent_and_no_trailing_zeros() {
        assert_eq!(number(0.25).unwrap(), "0.25");
        assert_eq!(number(2.0).unwrap(), "2");
        assert_eq!(number(-0.0).unwrap(), "0");
        assert_eq!(number(1e-9).unwrap(), "0");
        assert_eq!(number(1234.5678).unwrap(), "1234.5678");
        assert_eq!(number(f64::NAN), Err(WriteError::NotFinite));
    }

    #[test]
    fn inch_files_and_other_formats_are_written_as_asked() {
        let mut file = parse("%FSLAX24Y24*%\n%MOIN*%\n%ADD10C,0.01*%\nD10*\nX10000Y20000D02*\nX30000Y20000D01*\nM02*\n");
        assert!(file.is_clean());
        let text = write_ok(&file);
        assert!(text.contains("%FSLAX24Y24*%\n%MOIN*%\n"), "{text}");
        let again = parse(&text);
        assert_eq!(ops(&again), ops(&file));
        // The options override the file.
        file.units = Some(Units::Millimetres);
        let out = write(
            &file,
            &GerberOptions {
                format: Some(DEFAULT_FORMAT),
                units: Some(Units::Inches),
                comments: vec![],
            },
        )
        .unwrap();
        assert!(out.text.contains("%FSLAX46Y46*%\n%MOIN*%\n"));
    }

    #[test]
    fn apertures_macros_and_step_and_repeat_survive() {
        let mut file = parse(SAMPLE);
        file.macros.insert(
            "RB".into(),
            MacroDefinition {
                name: "RB".into(),
                body: vec!["21,1,$1,$2,0,0,0".into()],
            },
        );
        file.apertures.insert(
            14,
            Aperture::Macro {
                name: "RB".into(),
                params: vec![1.0, 0.5],
            },
        );
        file.apertures.insert(
            15,
            Aperture::Circle {
                diameter: 0.8,
                hole: Some(0.3),
            },
        );
        let text = write_ok(&file);
        let again = parse(&text);
        assert!(again.is_clean(), "{:?}\n{text}", again.diagnostics);
        assert_eq!(again.macros, file.macros);
        assert_eq!(again.apertures, file.apertures);

        let sr = parse(
            "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.1*%\n%SRX3Y2I5.5J4*%\nD10*\nX0Y0D03*\n%SR*%\nM02*\n",
        );
        assert!(sr.is_clean(), "{:?}", sr.diagnostics);
        let sr_text = write_ok(&sr);
        assert!(sr_text.contains("%SRX3Y2I5.5J4*%\n"), "{sr_text}");
        assert_eq!(ops(&parse(&sr_text)), ops(&sr));
    }

    #[test]
    fn a_missing_move_is_added_and_modal_codes_are_not_repeated() {
        let mut file = parse(SAMPLE);
        // Hand-made model: a line that starts away from the pen.
        file.commands.clear();
        let p = |x: f64, y: f64| Point { x, y };
        for (i, op) in [
            Operation::SelectAperture(10),
            Operation::Line {
                from: p(0.0, 0.0),
                to: p(1.0, 0.0),
            },
            Operation::Line {
                from: p(1.0, 0.0),
                to: p(1.0, 1.0),
            },
            Operation::Line {
                from: p(5.0, 5.0),
                to: p(6.0, 5.0),
            },
        ]
        .into_iter()
        .enumerate()
        {
            file.commands
                .push(eh3d_parser_gerber::Command { line: i + 1, op });
        }
        let text = write_ok(&file);
        let body = text.split("D10*\n").nth(1).unwrap();
        assert_eq!(
            body,
            "X0Y0D02*\nG01*\nX1000000Y0D01*\nX1000000Y1000000D01*\nX5000000Y5000000D02*\nX6000000Y5000000D01*\nM02*\n"
        );
    }

    #[test]
    fn what_cannot_be_written_is_an_error() {
        let mut file = parse(SAMPLE);
        let bad_format = CoordinateFormat {
            zero_omission: ZeroOmission::Trailing,
            ..DEFAULT_FORMAT
        };
        assert_eq!(
            write(
                &file,
                &GerberOptions {
                    format: Some(bad_format),
                    ..Default::default()
                }
            ),
            Err(WriteError::UnsupportedZeroOmission)
        );
        let no_digits = CoordinateFormat {
            decimal_digits: 0,
            ..DEFAULT_FORMAT
        };
        assert_eq!(
            write(
                &file,
                &GerberOptions {
                    format: Some(no_digits),
                    ..Default::default()
                }
            ),
            Err(WriteError::InvalidFormat)
        );
        let big = CoordinateFormat {
            integer_digits: 7,
            decimal_digits: 6,
            ..DEFAULT_FORMAT
        };
        assert_eq!(
            write(
                &file,
                &GerberOptions {
                    format: Some(big),
                    ..Default::default()
                }
            ),
            Err(WriteError::InvalidFormat)
        );
        assert_eq!(
            write(
                &file,
                &GerberOptions {
                    comments: vec!["a*b".into()],
                    ..Default::default()
                }
            ),
            Err(WriteError::ForbiddenCharacter)
        );
        file.commands.push(eh3d_parser_gerber::Command {
            line: 99,
            op: Operation::SelectAperture(77),
        });
        assert_eq!(
            write(&file, &GerberOptions::default()),
            Err(WriteError::BadAperture(77))
        );
        file.commands.pop();
        file.commands.push(eh3d_parser_gerber::Command {
            line: 99,
            op: Operation::Flash {
                at: Point { x: 10000.0, y: 0.0 },
            },
        });
        assert_eq!(
            write(&file, &GerberOptions::default()),
            Err(WriteError::CoordinateOverflow)
        );
        file.commands.pop();
        file.apertures.insert(
            20,
            Aperture::Macro {
                name: "NOPE".into(),
                params: vec![],
            },
        );
        assert_eq!(
            write(&file, &GerberOptions::default()),
            Err(WriteError::MissingMacro("NOPE".into()))
        );
        file.apertures.remove(&20);
        file.apertures.insert(
            21,
            Aperture::Polygon {
                diameter: 1.0,
                vertices: 2,
                rotation: 0.0,
                hole: None,
            },
        );
        assert_eq!(
            write(&file, &GerberOptions::default()),
            Err(WriteError::BadPolygon)
        );
        file.apertures.remove(&21);
        file.apertures.insert(
            3,
            Aperture::Circle {
                diameter: 1.0,
                hole: None,
            },
        );
        assert_eq!(
            write(&file, &GerberOptions::default()),
            Err(WriteError::BadAperture(3))
        );
    }

    #[test]
    fn attributes_other_than_file_attributes_are_counted_as_dropped() {
        let mut file = parse(SAMPLE);
        file.attributes.push(Attribute {
            kind: "TA".into(),
            name: ".AperFunction".into(),
            values: vec!["Conductor".into()],
        });
        file.attributes.push(Attribute {
            kind: "TD".into(),
            name: String::new(),
            values: vec![],
        });
        let out = write(&file, &GerberOptions::default()).unwrap();
        assert_eq!(out.dropped_attributes, 2);
        assert!(!out.text.contains("AperFunction"));
    }

    #[test]
    fn an_empty_file_is_a_valid_file() {
        let file = parse("");
        let text = write_ok(&file);
        assert_eq!(text, "%FSLAX46Y46*%\n%MOMM*%\nM02*\n");
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
            // a multiple of one micrometre inside +-5000 mm
            ((self.next() % 10_000_000_001) as i64 - 5_000_000_000) as f64 / 1_000_000.0
        }
    }

    #[test]
    fn generated_drawings_survive_write_and_read() {
        let mut rng = Seeded(7);
        for _ in 0..50 {
            let mut file = parse(
                "%FSLAX46Y46*%
%MOMM*%
%ADD10C,0.1*%
%ADD11R,0.5X0.5*%
",
            );
            let start = Point {
                x: rng.coordinate(),
                y: rng.coordinate(),
            };
            let mut at = Some(start);
            let mut commands = vec![Operation::SelectAperture(10), Operation::Move { to: start }];
            for _ in 0..40 {
                let p = Point {
                    x: rng.coordinate(),
                    y: rng.coordinate(),
                };
                match rng.next() % 4 {
                    0 => {
                        commands.push(Operation::Move { to: p });
                        at = Some(p);
                    }
                    1 => {
                        commands.push(Operation::Flash { at: p });
                        at = Some(p);
                    }
                    2 => {
                        let from = at.unwrap_or(Point { x: 0.0, y: 0.0 });
                        commands.push(Operation::Line { from, to: p });
                        at = Some(p);
                    }
                    _ => commands.push(Operation::SelectAperture(10 + (rng.next() % 2) as u32)),
                }
            }
            file.commands = commands
                .into_iter()
                .enumerate()
                .map(|(i, op)| eh3d_parser_gerber::Command { line: i + 1, op })
                .collect();
            let text = write_ok(&file);
            let again = parse(&text);
            assert!(
                again.is_clean(),
                "{:?}
{text}",
                again.diagnostics
            );
            assert_eq!(ops(&again), ops(&file), "{text}");
            assert_eq!(write_ok(&again), text, "writing twice is stable");
        }
    }
}
