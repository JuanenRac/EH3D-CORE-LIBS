//! The format, the zeros and the units that a caller assumes for a file that does not declare them.

use eh3d_parser_excellon::{
    codes, parse, parse_bytes, parse_bytes_with_format, parse_with_format, ExcellonDefaults,
    ExcellonFile, Severity, Units, ZeroSuppression,
};
use std::path::PathBuf;

fn holes(file: &ExcellonFile) -> Vec<(u32, f64, f64)> {
    file.drills
        .iter()
        .map(|h| (h.tool, h.at.x, h.at.y))
        .collect()
}

fn format_2_4(zeros: ZeroSuppression) -> ExcellonDefaults {
    ExcellonDefaults {
        default_format: Some((2, 4)),
        default_zeros: Some(zeros),
        ..ExcellonDefaults::default()
    }
}

#[test]
fn the_default_format_reads_a_file_with_trailing_zeros_present() {
    let text = "M48\nMETRIC\nT1C0.8\n%\nT1\nX15000Y5000\nX25000Y10000\nM30\n";
    let file = parse_with_format(text, &format_2_4(ZeroSuppression::Leading));
    assert_eq!(holes(&file), [(1, 1.5, 0.5), (1, 2.5, 1.0)]);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    // the same file with no defaults is read as 3:3
    assert_eq!(holes(&parse(text)), [(1, 15.0, 5.0), (1, 25.0, 10.0)]);
}

#[test]
fn the_default_format_reads_a_file_with_leading_zeros_present() {
    let text = "M48\nMETRIC\nT1C0.8\n%\nT1\nX15Y2\nX0001Y5\nM30\n";
    let file = parse_with_format(text, &format_2_4(ZeroSuppression::Trailing));
    assert_eq!(holes(&file), [(1, 15.0, 20.0), (1, 0.01, 50.0)]);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
}

#[test]
fn what_the_file_declares_wins_over_the_defaults() {
    // a comment with the format beats the default format
    let comment = "; FILE_FORMAT=3:3\nM48\nMETRIC\nT1C0.8\n%\nT1\nX1500Y500\nM30\n";
    let file = parse_with_format(comment, &format_2_4(ZeroSuppression::Leading));
    assert_eq!(holes(&file), [(1, 1.5, 0.5)]);
    // the format and the zeros of the header beat the defaults, and so do the units
    let header = "M48\nINCH,LZ,00.0000\nT1C0.03\n%\nT1\nX15Y2\nM30\n";
    let defaults = ExcellonDefaults {
        default_format: Some((3, 3)),
        default_zeros: Some(ZeroSuppression::Leading),
        default_units: Some(Units::Millimetres),
    };
    let file = parse_with_format(header, &defaults);
    assert_eq!(holes(&file), [(1, 15.0, 20.0)]);
    assert_eq!(file.units, Some(Units::Inches));
    assert_eq!(file.zeros, Some(ZeroSuppression::Trailing));
    assert_eq!(file, parse(header));
    // M72 beats the default units
    let old = "M48\nM72\nT1C0.03\n%\nT1\nX1.0Y1.0\nM30\n";
    let defaults = ExcellonDefaults {
        default_units: Some(Units::Millimetres),
        ..ExcellonDefaults::default()
    };
    assert_eq!(parse_with_format(old, &defaults).units, Some(Units::Inches));
}

#[test]
fn the_default_units_stand_in_for_missing_ones() {
    let text = "M48\nT1C0.8\n%\nT1\nX1.0Y1.0\nX2.0Y2.0\nM30\n";
    let plain = parse(text);
    assert_eq!(plain.units, None);
    assert!(plain
        .diagnostics
        .iter()
        .any(|d| d.code == codes::UNITS_MISSING));
    let defaults = ExcellonDefaults {
        default_units: Some(Units::Inches),
        ..ExcellonDefaults::default()
    };
    let file = parse_with_format(text, &defaults);
    assert_eq!(file.units, Some(Units::Inches));
    assert!(file
        .diagnostics
        .iter()
        .all(|d| d.code != codes::UNITS_MISSING && d.code != codes::UNITS_SUSPECT));
    // inches with nothing else said read the integer coordinates as 2:4
    let integer = "M48\nT1C0.03\n%\nT1\nX015000Y005000\nM30\n";
    let file = parse_with_format(integer, &defaults);
    assert_eq!(holes(&file), [(1, 1.5, 0.5)]);
}

#[test]
fn the_defaults_do_not_move_the_lines_of_the_diagnostics() {
    // line 6 has a coordinate with too many digits for 2:4, line 7 selects a tool never defined
    let text = "M48\nMETRIC\nT1C0.8\n%\nT1\nX1234567Y1\nT9\nM30\n";
    let file = parse_with_format(text, &format_2_4(ZeroSuppression::Leading));
    let lines: Vec<(usize, &str)> = file.diagnostics.iter().map(|d| (d.line, d.code)).collect();
    assert_eq!(
        lines,
        [(6, codes::NUMBER_TOO_LARGE), (7, codes::TOOL_UNDEFINED)]
    );
    assert!(file
        .diagnostics
        .iter()
        .all(|d| d.severity == Severity::Error));
    // the bytes entry point gives the same file
    let bytes = parse_bytes_with_format(text.as_bytes(), &format_2_4(ZeroSuppression::Leading))
        .expect("small file");
    assert_eq!(bytes, file);
}

#[test]
fn a_format_out_of_range_is_ignored_and_told() {
    let text = "M48\nMETRIC\nT1C0.8\n%\nT1\nX1.5Y2.5\nM30\n";
    for bad in [(0, 4), (9, 0), (2, 9), (0, 0)] {
        let defaults = ExcellonDefaults {
            default_format: Some(bad),
            ..ExcellonDefaults::default()
        };
        assert!(!defaults.is_valid(), "{bad:?}");
        let file = parse_with_format(text, &defaults);
        let first = &file.diagnostics[0];
        assert_eq!(
            (first.line, first.code, first.severity),
            (1, codes::DEFAULTS_INVALID, Severity::Warning),
            "{bad:?}"
        );
        // the rest is read as if there were no defaults
        let rest = ExcellonFile {
            diagnostics: file.diagnostics[1..].to_vec(),
            ..file.clone()
        };
        assert_eq!(rest, parse(text), "{bad:?}");
    }
    for good in [(1, 0), (8, 8), (2, 4)] {
        let defaults = ExcellonDefaults {
            default_format: Some(good),
            ..ExcellonDefaults::default()
        };
        assert!(defaults.is_valid(), "{good:?}");
        assert!(parse_with_format(text, &defaults)
            .diagnostics
            .iter()
            .all(|d| d.code != codes::DEFAULTS_INVALID));
    }
    assert!(ExcellonDefaults::default().is_valid());
}

#[test]
fn no_defaults_read_the_example_files_as_before() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/excellon");
    let mut seen = 0;
    for entry in std::fs::read_dir(dir).expect("the golden folder") {
        let path = entry.expect("an entry").path();
        match path.extension() {
            Some(ext) if ext != "expected" => {}
            _ => continue,
        }
        let bytes = std::fs::read(&path).expect("a file");
        let want = parse_bytes(&bytes).expect("small file");
        let got =
            parse_bytes_with_format(&bytes, &ExcellonDefaults::default()).expect("small file");
        assert_eq!(got, want, "{}", path.display());
        let text = String::from_utf8_lossy(&bytes).into_owned();
        assert_eq!(
            parse_with_format(&text, &ExcellonDefaults::default()),
            parse(&text),
            "{}",
            path.display()
        );
        seen += 1;
    }
    assert!(seen >= 6, "the corpus is missing");
}
