//! Reading bytes: the size limit, Latin-1 text and the byte order mark; and the stable codes of the diagnostics.

use eh3d_parser_gcode::{
    codes, interpret, parse, parse_bytes, parse_bytes_with, Dialect, InputError, InputLimits,
    Severity,
};

const SAMPLE: &[u8] = b"G21 G90\nG1 X10 Y10 F100\nM2\n";

#[test]
fn a_file_above_the_limit_is_refused() {
    let limits = InputLimits { max_bytes: 16 };
    let big = SAMPLE.repeat(4);
    assert!(big.len() > 16);
    assert_eq!(
        parse_bytes_with(&big, Dialect::Grbl, &limits).err(),
        Some(InputError::TooLarge {
            size: big.len(),
            limit: 16
        })
    );
    // the same bytes pass with the default limit
    assert!(parse_bytes(&big, Dialect::Grbl).is_ok());
}

#[test]
fn the_default_limit_is_256_mib() {
    assert_eq!(InputLimits::default().max_bytes, 256 * 1024 * 1024);
}

#[test]
fn latin1_text_is_read_with_a_warning() {
    let mut bytes = b"; caf\xe9\n".to_vec();
    bytes.extend_from_slice(SAMPLE);
    let diagnostics = parse_bytes(&bytes, Dialect::Grbl).unwrap().diagnostics;
    let first = &diagnostics[0];
    assert_eq!(first.code, codes::INPUT_NOT_UTF8);
    assert_eq!(first.severity, Severity::Warning);
    assert_eq!(first.line, 1);
}

#[test]
fn utf8_with_a_byte_order_mark_has_no_encoding_warning() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(SAMPLE);
    let diagnostics = parse_bytes(&bytes, Dialect::Grbl).unwrap().diagnostics;
    assert!(diagnostics.iter().all(|d| d.code != codes::INPUT_NOT_UTF8));
    let mut utf8 = b"; caf".to_vec();
    utf8.extend_from_slice("\u{e9}\n".as_bytes());
    utf8.extend_from_slice(SAMPLE);
    let diagnostics = parse_bytes(&utf8, Dialect::Grbl).unwrap().diagnostics;
    assert!(diagnostics.iter().all(|d| d.code != codes::INPUT_NOT_UTF8));
}

#[test]
fn diagnostics_carry_stable_codes() {
    let program = parse(
        "G1 X99999999999999999999\nG1 X1 F-5\n(open\n",
        Dialect::Grbl,
    );
    let trace = interpret(&program);
    let found: Vec<&str> = trace.diagnostics.iter().map(|d| d.code).collect();
    assert!(found.contains(&codes::NUMBER_TOO_LARGE), "{found:?}");
    assert!(found.contains(&codes::COMMENT_UNCLOSED), "{found:?}");
    assert!(found.contains(&codes::NEGATIVE_FEED), "{found:?}");
}

#[test]
fn a_number_has_a_limit() {
    let program = parse(
        "G1 X1000000000000000\nG1 X10000000000000000\n",
        Dialect::Grbl,
    );
    let has_x = |i: usize| program.blocks[i].words.iter().any(|w| w.letter == 'X');
    assert!(has_x(0), "1e15 is accepted");
    assert!(!has_x(1), "1e16 is not");
    assert_eq!(program.diagnostics.len(), 1);
    assert_eq!(program.diagnostics[0].code, codes::NUMBER_TOO_LARGE);
    assert_eq!(program.diagnostics[0].line, 2);
}
