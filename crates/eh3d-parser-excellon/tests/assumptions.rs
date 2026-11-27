//! What the reader has to suppose about a drill file is told, once, and can be refused; and nothing that is not a
//! hole becomes one, or the other way round, without a diagnostic.

use eh3d_parser_excellon::{
    codes, parse, parse_with_format, parse_with_limits, parse_with_units, ExcellonDefaults,
    ExcellonFile, ExcellonLimits, Severity, Units, ZeroSuppression, MAX_DIAGNOSTICS,
};

fn holes(file: &ExcellonFile) -> Vec<(f64, f64)> {
    file.drills.iter().map(|h| (h.at.x, h.at.y)).collect()
}

fn told(file: &ExcellonFile, code: &str) -> Vec<(usize, Severity)> {
    file.diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| (d.line, d.severity))
        .collect()
}

fn body(header: &str, lines: &str) -> String {
    format!("M48\n{header}\nT1C0.8\n%\nT1\n{lines}\nM30\n")
}

#[test]
fn integer_coordinates_with_no_format_are_told_once_and_read_as_before() {
    for (header, x) in [("METRIC", 12.5), ("INCH", 1.25), ("METRIC,TZ", 12.5)] {
        let file = parse(&body(header, "X012500Y012500\nX012500Y0"));
        assert_eq!(holes(&file)[0], (x, x), "{header}");
        assert_eq!(
            told(&file, codes::FORMAT_ASSUMED),
            [(6, Severity::Warning)],
            "{header}"
        );
        // six digits are the whole of the format: the zeros change nothing
        assert!(told(&file, codes::ZEROS_ASSUMED).is_empty(), "{header}");
        assert!(file.is_clean(), "{:?}", file.diagnostics);
    }
}

#[test]
fn short_coordinates_with_no_zero_suppression_are_told() {
    let file = parse(&body("METRIC", "X1500Y500\nX2500Y500"));
    assert_eq!(holes(&file), [(1.5, 0.5), (2.5, 0.5)]);
    assert_eq!(told(&file, codes::ZEROS_ASSUMED), [(6, Severity::Warning)]);
    assert_eq!(told(&file, codes::FORMAT_ASSUMED).len(), 1);
    // a leading zero speaks against the supposition, and the message says it
    let file = parse(&body("M72", "X0125Y0125"));
    assert_eq!(holes(&file), [(0.0125, 0.0125)]);
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.code == codes::ZEROS_ASSUMED && d.message.contains("trailing")));
}

#[test]
fn what_the_file_or_the_caller_says_is_not_supposed() {
    let quiet = [
        body("METRIC,TZ,000.000", "X1500Y500"),
        body("METRIC,LZ,000.000", "X0015Y005"),
        format!(";FILE_FORMAT=3:3\n{}", body("METRIC,TZ", "X1500Y500")),
        // a point, and a zero, mean the same under any format
        body("METRIC", "X1.5Y0\nX0Y2.5"),
    ];
    for text in &quiet {
        let file = parse(text);
        assert_eq!(file.assumptions().count(), 0, "{text}");
    }
    let defaults = ExcellonDefaults {
        default_format: Some((3, 3)),
        default_zeros: Some(ZeroSuppression::Leading),
        default_units: Some(Units::Millimetres),
    };
    let file = parse_with_format("M48\nT1C0.8\n%\nT1\nX1500Y500\nM30\n", &defaults);
    assert_eq!(file.assumptions().count(), 0, "{:?}", file.diagnostics);
    // the format alone declared: only the zeros are supposed
    let file = parse(&body("METRIC,000.000", "X1500Y500"));
    assert!(told(&file, codes::FORMAT_ASSUMED).is_empty());
    assert_eq!(told(&file, codes::ZEROS_ASSUMED).len(), 1);
}

#[test]
fn a_program_that_must_not_guess_refuses_the_suppositions() {
    let mut file = parse(&body("METRIC", "X1500Y500"));
    assert!(file.is_clean());
    let before = holes(&file);
    assert!(file.refuse_assumptions());
    assert!(!file.is_clean());
    assert_eq!(holes(&file), before);
    assert!(file.assumptions().all(|d| d.severity == Severity::Error));
    // no units counts only when there is something to place
    let mut file = parse("M48\nT1C0.8\n%\nT1\nX1.0Y1.0\nM30\n");
    assert!(file.refuse_assumptions());
    assert_eq!(told(&file, codes::UNITS_MISSING), [(1, Severity::Error)]);
    let mut empty = parse("M48\n%\nM30\n");
    assert!(!empty.refuse_assumptions());
    assert!(empty.is_clean());
}

#[test]
fn units_given_by_the_caller_choose_the_format_of_a_file_without_units() {
    let text = "M48\nT1C0.03\n%\nT1\nX010000Y020000\nM30\n";
    let inches = parse_with_units(text, Units::Inches);
    assert_eq!(holes(&inches), [(1.0, 2.0)]);
    assert_eq!(told(&inches, codes::FORMAT_ASSUMED).len(), 1);
    assert!(told(&inches, codes::UNITS_MISSING).is_empty());
    let mm = parse_with_units(text, Units::Millimetres);
    assert_eq!(holes(&mm), [(10.0, 20.0)]);
}

#[test]
fn a_zero_set_and_the_position_of_the_end_are_not_holes() {
    let file = parse(&body("METRIC", "G93X0Y0\nG92X0Y0\nX1.0Y1.0"));
    assert_eq!(holes(&file), [(1.0, 1.0)]);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let file = parse(&body("METRIC", "G93X5.0Y0\nX1.0Y1.0"));
    assert_eq!(holes(&file), [(1.0, 1.0)]);
    assert_eq!(
        told(&file, codes::GEOMETRY_COMMAND_UNSUPPORTED),
        [(6, Severity::Error)]
    );
    for end in ["M30X0Y0", "M00X10.0Y10.0"] {
        let file = parse(&format!("M48\nMETRIC\nT1C0.8\n%\nT1\nX1.0Y1.0\n{end}\n"));
        assert_eq!(holes(&file), [(1.0, 1.0)], "{end}");
        assert_eq!(
            told(&file, codes::COMMAND_IGNORED),
            [(7, Severity::Warning)],
            "{end}"
        );
    }
}

#[test]
fn what_is_left_out_of_a_line_or_written_twice_is_an_error() {
    for (line, code) in [
        ("1.5Y2.0", codes::CHARACTER_IGNORED),
        ("X1.0X2.0Y1.0", codes::WORD_REPEATED),
        ("X1.0Y1.0G85X2.0X3.0", codes::WORD_REPEATED),
        ("X1.0Y1.0G85", codes::COORDINATE_MISSING),
    ] {
        let file = parse(&body("METRIC", line));
        assert_eq!(told(&file, code), [(6, Severity::Error)], "{line}");
    }
    // a header that is never closed: its coordinates are not drilled, and it is said
    let file = parse("M48\nMETRIC\nT1C0.8\nT1\nX1.0Y1.0\nM30\n");
    assert!(file.drills.is_empty());
    assert!(!file.is_clean());
}

#[test]
fn units_declared_twice_and_differently_are_an_error() {
    let file = parse(&body("METRIC\nM72", "X1.0Y1.0"));
    assert_eq!(told(&file, codes::UNITS_CONFLICT), [(3, Severity::Error)]);
    let file = parse(&body("INCH", "M71\nX1.0Y1.0"));
    assert_eq!(told(&file, codes::UNITS_CONFLICT), [(6, Severity::Error)]);
    assert!(told(
        &parse(&body("METRIC\nM71", "X1.0Y1.0")),
        codes::UNITS_CONFLICT
    )
    .is_empty());
}

#[test]
fn blanks_in_the_header_do_not_lose_the_zeros_and_a_mask_without_a_point_is_told() {
    let file = parse(&body("INCH , LZ , 00.0000", "X01Y02"));
    assert_eq!(file.units, Some(Units::Inches));
    assert_eq!(file.zeros, Some(ZeroSuppression::Trailing));
    assert_eq!(holes(&file), [(1.0, 2.0)]);
    assert_eq!(file.assumptions().count(), 0, "{:?}", file.diagnostics);
    for text in [
        body("METRIC,000000", "X1.0Y1.0"),
        format!(";FILE_FORMAT=33\n{}", body("METRIC", "X1.0Y1.0")),
    ] {
        assert_eq!(
            told(&parse(&text), codes::FORMAT_COMMENT_INVALID).len(),
            1,
            "{text}"
        );
    }
}

#[test]
fn slots_count_for_the_limit_and_the_diagnostics_are_bounded() {
    let limits = ExcellonLimits {
        max_items: 2,
        ..ExcellonLimits::default()
    };
    let slot = "X1.0Y1.0G85X2.0Y1.0\n".repeat(5);
    let file = parse_with_limits(&body("METRIC", &slot), &limits);
    assert_eq!(file.slots.len(), 2);
    assert_eq!(told(&file, codes::LIMIT_EXCEEDED).len(), 1);
    let noise = "Q1\n".repeat(MAX_DIAGNOSTICS + 10);
    let file = parse(&body("METRIC", &format!("{noise}X1500Y500")));
    assert!(file.diagnostics.len() <= MAX_DIAGNOSTICS + 5);
    assert_eq!(told(&file, codes::TOO_MANY_DIAGNOSTICS).len(), 1);
    // what was supposed is told even past the bound
    assert_eq!(told(&file, codes::FORMAT_ASSUMED).len(), 1);
}
