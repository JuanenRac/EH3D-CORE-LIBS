//! The shared rules of the readers, from outside: numbers, coordinates, units and the bytes of a file.

use eh3d_parser_common::{
    codes, coordinate_error_code, decode_coordinate, decode_input, parse_f64, parse_number,
    resolve_units, Diagnostic, InputError, InputLimits, NumberError, Severity, Units,
    ZeroSuppression, ASSUMED_UNITS, MAX_NUMBER_MAGNITUDE,
};

#[test]
fn a_number_is_accepted_up_to_the_magnitude_limit_and_not_a_hair_beyond() {
    assert_eq!(parse_number("1e15"), Ok(1e15));
    assert_eq!(parse_number("-1e15"), Ok(-1e15));
    assert_eq!(parse_number("1.0000001e15"), Err(NumberError::TooLarge));
    assert_eq!(parse_number("-1.0000001e15"), Err(NumberError::TooLarge));
    assert_eq!(MAX_NUMBER_MAGNITUDE, 1e15);
    assert_eq!(
        parse_number("1e400"),
        Err(NumberError::TooLarge),
        "beyond a double is too large, not invalid"
    );
    assert_eq!(parse_f64("1e16"), None);
}

#[test]
fn text_that_is_not_a_plain_decimal_number_is_invalid() {
    for bad in [
        "", " ", "-", "+", ".", "1.2.3", "1,5", "0x10", "1_000", "--5", "5-", "١٢", "NaN", "nan",
        "1e", "e5",
    ] {
        assert_eq!(parse_number(bad), Err(NumberError::Invalid), "{bad:?}");
    }
    for (good, want) in [
        ("+5", 5.0),
        (" 5 ", 5.0),
        (".5", 0.5),
        ("5.", 5.0),
        ("-0.0", 0.0),
        ("1E3", 1000.0),
        ("007", 7.0),
    ] {
        assert_eq!(parse_number(good), Ok(want), "{good:?}");
    }
}

#[test]
fn the_zero_suppression_decides_where_the_missing_zeros_were() {
    // format 2.4: six digits in all
    let lead = |t: &str| decode_coordinate(t, 2, 4, ZeroSuppression::Leading);
    let trail = |t: &str| decode_coordinate(t, 2, 4, ZeroSuppression::Trailing);
    assert_eq!(lead("0"), Some(0.0));
    assert_eq!(lead("1"), Some(0.0001));
    assert_eq!(lead("123456"), Some(12.3456));
    assert_eq!(lead("1234567"), None, "one digit too many");
    assert_eq!(trail("1"), Some(10.0));
    assert_eq!(trail("12"), Some(12.0));
    assert_eq!(trail("123456"), Some(12.3456));
    assert_eq!(trail("1234567"), None);
    assert_eq!(trail("-5"), Some(-50.0));
    assert_eq!(trail("+5"), Some(50.0));
}

#[test]
fn a_decimal_point_overrides_the_format_and_a_sign_alone_is_nothing() {
    for z in [ZeroSuppression::Leading, ZeroSuppression::Trailing] {
        assert_eq!(decode_coordinate("12.5", 2, 4, z), Some(12.5));
        assert_eq!(decode_coordinate("-.5", 2, 4, z), Some(-0.5));
        assert_eq!(
            decode_coordinate("123456.789", 2, 4, z),
            Some(123456.789),
            "a decimal value is not limited by the number of digits"
        );
        assert_eq!(decode_coordinate("", 2, 4, z), None);
        assert_eq!(decode_coordinate("-", 2, 4, z), None);
        assert_eq!(decode_coordinate("+", 2, 4, z), None);
        assert_eq!(decode_coordinate("--5", 2, 4, z), None);
        assert_eq!(
            decode_coordinate("1e3", 2, 4, z),
            None,
            "an exponent without a point is not a coordinate"
        );
        assert_eq!(decode_coordinate("12 5", 2, 4, z), None);
        assert_eq!(decode_coordinate("1.5e999", 2, 4, z), None);
    }
}

#[test]
fn a_format_with_no_decimals_reads_whole_numbers() {
    assert_eq!(
        decode_coordinate("42", 3, 0, ZeroSuppression::Leading),
        Some(42.0)
    );
    assert_eq!(
        decode_coordinate("4", 3, 0, ZeroSuppression::Trailing),
        Some(400.0)
    );
    assert_eq!(
        decode_coordinate("4444", 3, 0, ZeroSuppression::Leading),
        None
    );
}

#[test]
fn the_error_code_of_a_refused_coordinate_tells_too_many_digits_from_garbage() {
    assert_eq!(
        coordinate_error_code("1234567", 2, 4),
        codes::NUMBER_TOO_LARGE
    );
    assert_eq!(
        coordinate_error_code("-1234567", 2, 4),
        codes::NUMBER_TOO_LARGE
    );
    assert_eq!(
        coordinate_error_code("+1234567", 2, 4),
        codes::NUMBER_TOO_LARGE
    );
    assert_eq!(coordinate_error_code("1e99", 2, 4), codes::NUMBER_TOO_LARGE);
    assert_eq!(
        coordinate_error_code("123456", 2, 4),
        codes::COORDINATE_INVALID,
        "it fits: whatever was wrong, it was not the size"
    );
    for garbage in ["", "-", "12a", "1.2.3", "abc"] {
        assert_eq!(
            coordinate_error_code(garbage, 2, 4),
            codes::COORDINATE_INVALID,
            "{garbage:?}"
        );
    }
}

#[test]
fn a_file_with_no_units_is_millimetres_with_one_warning_and_a_declared_unit_adds_nothing() {
    let mut d = vec![Diagnostic::new(
        1,
        Severity::Info,
        codes::INPUT_NOT_UTF8,
        "earlier",
    )];
    assert_eq!(resolve_units(Some(Units::Inches), 3, &mut d), Units::Inches);
    assert_eq!(d.len(), 1);
    assert_eq!(resolve_units(None, 7, &mut d), ASSUMED_UNITS);
    assert_eq!(d.len(), 2, "the warning is added after what was there");
    assert_eq!(
        (d[1].line, d[1].severity, d[1].code),
        (7, Severity::Warning, codes::UNITS_MISSING)
    );
    assert_eq!(Units::Inches.to_mm(2.0), 50.8);
    assert_eq!(Units::Millimetres.to_mm(2.0), 2.0);
}

#[test]
fn the_size_limit_is_exact_and_the_message_gives_both_numbers() {
    let limits = InputLimits { max_bytes: 4 };
    assert!(decode_input(b"abcd", &limits).is_ok());
    let e = decode_input(b"abcde", &limits).unwrap_err();
    assert_eq!(e, InputError::TooLarge { size: 5, limit: 4 });
    let shown = e.to_string();
    assert!(shown.contains('5') && shown.contains('4'), "{shown}");
    assert!(decode_input(b"", &InputLimits { max_bytes: 0 }).is_ok());
    assert!(decode_input(b"x", &InputLimits { max_bytes: 0 }).is_err());
}

#[test]
fn the_encoding_of_a_file_is_utf8_with_or_without_a_mark_or_else_latin1_with_one_warning() {
    let limits = InputLimits::default();
    let plain = decode_input("año ✓".as_bytes(), &limits).unwrap();
    assert_eq!((plain.text.as_str(), plain.diagnostics.len()), ("año ✓", 0));
    let mut marked = vec![0xEF, 0xBB, 0xBF];
    marked.extend_from_slice("año".as_bytes());
    assert_eq!(decode_input(&marked, &limits).unwrap().text, "año");
    assert_eq!(decode_input(&[0xEF, 0xBB, 0xBF], &limits).unwrap().text, "");
    // Latin-1: every byte is the character with the same number, nothing becomes U+FFFD
    let latin = decode_input(&[b'c', b'a', b'f', 0xE9, 0xFF, 0x80], &limits).unwrap();
    assert_eq!(latin.text, "caf\u{e9}\u{ff}\u{80}");
    assert_eq!(latin.diagnostics.len(), 1);
    assert_eq!(
        (latin.diagnostics[0].line, latin.diagnostics[0].code),
        (1, codes::INPUT_NOT_UTF8)
    );
    // two bytes that start like a mark but are not one
    let lookalike = decode_input(&[0xEF, 0xBB, 0x41], &limits).unwrap();
    assert_eq!(lookalike.text, "\u{ef}\u{bb}A");
    // a mark in front of bytes that are not UTF-8: the mark goes, the rest is Latin-1
    let both = decode_input(&[0xEF, 0xBB, 0xBF, 0xE9], &limits).unwrap();
    assert_eq!(both.text, "\u{e9}");
    assert_eq!(both.diagnostics.len(), 1);
    assert_eq!(decode_input(b"", &limits).unwrap().text, "");
}

#[test]
fn the_codes_of_the_diagnostics_are_the_ones_the_tools_filter_on() {
    for code in [
        codes::UNITS_MISSING,
        codes::INPUT_NOT_UTF8,
        codes::NUMBER_TOO_LARGE,
        codes::COORDINATE_INVALID,
    ] {
        assert!(codes::ALL.contains(&code), "{code}");
    }
    assert!(Severity::Error > Severity::Warning && Severity::Warning > Severity::Info);
}
