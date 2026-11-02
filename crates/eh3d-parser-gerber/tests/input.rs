//! Reading bytes: the size limit, Latin-1 text and the byte order mark; and the stable codes of the diagnostics.

use eh3d_parser_gerber::{
    codes, parse, parse_bytes, parse_bytes_with, InputError, InputLimits, Severity, Units,
};

const SAMPLE: &[u8] =
    b"%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.2*%\nD10*\nX1000Y1000D02*\nX2000Y1000D01*\nM02*\n";

#[test]
fn a_file_above_the_limit_is_refused() {
    let limits = InputLimits { max_bytes: 16 };
    let big = SAMPLE.repeat(4);
    assert!(big.len() > 16);
    assert_eq!(
        parse_bytes_with(&big, &limits).err(),
        Some(InputError::TooLarge {
            size: big.len(),
            limit: 16
        })
    );
    // the same bytes pass with the default limit
    assert!(parse_bytes(&big).is_ok());
}

#[test]
fn the_default_limit_is_256_mib() {
    assert_eq!(InputLimits::default().max_bytes, 256 * 1024 * 1024);
}

#[test]
fn latin1_text_is_read_with_a_warning() {
    let mut bytes = b"G04 caf\xe9\n".to_vec();
    bytes.extend_from_slice(SAMPLE);
    let diagnostics = parse_bytes(&bytes).unwrap().diagnostics;
    let first = &diagnostics[0];
    assert_eq!(first.code, codes::INPUT_NOT_UTF8);
    assert_eq!(first.severity, Severity::Warning);
    assert_eq!(first.line, 1);
}

#[test]
fn utf8_with_a_byte_order_mark_has_no_encoding_warning() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(SAMPLE);
    let diagnostics = parse_bytes(&bytes).unwrap().diagnostics;
    assert!(diagnostics.iter().all(|d| d.code != codes::INPUT_NOT_UTF8));
    let mut utf8 = b"G04 caf".to_vec();
    utf8.extend_from_slice("\u{e9}\n".as_bytes());
    utf8.extend_from_slice(SAMPLE);
    let diagnostics = parse_bytes(&utf8).unwrap().diagnostics;
    assert!(diagnostics.iter().all(|d| d.code != codes::INPUT_NOT_UTF8));
}

#[test]
fn a_file_without_units_gets_one_warning_and_millimetres() {
    let file = parse_bytes(b"%FSLAX24Y24*%\n%ADD10C,0.2*%\nD10*\nX1000Y1000D02*\nM02*\n").unwrap();
    let missing: Vec<_> = file
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::UNITS_MISSING)
        .collect();
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].severity, Severity::Warning);
    assert_eq!(file.units, None);
    assert_eq!(file.effective_units(), Units::Millimetres);
    assert!(file.is_clean(), "a missing unit is a warning, not an error");
}

#[test]
fn a_file_with_units_has_no_units_warning() {
    for header in ["%MOMM*%", "%MOIN*%", "G70*", "G71*"] {
        let text = format!("%FSLAX24Y24*%\n{header}\nM02*\n");
        let file = parse(&text);
        assert!(
            file.diagnostics
                .iter()
                .all(|d| d.code != codes::UNITS_MISSING),
            "{header}"
        );
        assert_eq!(file.effective_units(), file.units.unwrap());
    }
}

#[test]
fn diagnostics_carry_stable_codes() {
    let file = parse("%FSLAX24Y24*%\n%MOMM*%\nG91*\nG99*\nX9999999999Y0D01*\n");
    let found: Vec<&str> = file.diagnostics.iter().map(|d| d.code).collect();
    assert!(found.contains(&codes::INCREMENTAL_UNSUPPORTED), "{found:?}");
    assert!(found.contains(&codes::UNKNOWN_G_CODE), "{found:?}");
    assert!(found.contains(&codes::NUMBER_TOO_LARGE), "{found:?}");
    assert!(found.contains(&codes::END_MISSING), "{found:?}");
}
