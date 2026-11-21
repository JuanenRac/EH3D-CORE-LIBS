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

#[test]
fn a_file_above_the_limit_is_refused_by_its_size_before_it_is_read() {
    use eh3d_parser_common::{read_file_limited, ReadError};
    let dir = std::env::temp_dir().join(format!("eh3d_limit_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("layer.gbr");
    std::fs::write(&path, b"0123456789").unwrap();
    let ok = read_file_limited(&path, &InputLimits { max_bytes: 10 }).unwrap();
    assert_eq!(ok.len(), 10);
    match read_file_limited(&path, &InputLimits { max_bytes: 9 }) {
        Err(ReadError::TooLarge(InputError::TooLarge { size, limit })) => {
            assert_eq!((size, limit), (10, 9))
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        read_file_limited(&dir.join("missing.gbr"), &InputLimits::default()),
        Err(ReadError::Io(_))
    ));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_limit_of_usize_max_reads_the_file_and_does_not_overflow() {
    use eh3d_parser_common::{read_file_limited, InputLimits};
    let dir = std::env::temp_dir().join("eh3d-limit-max");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.gbr");
    std::fs::write(&path, b"0123456789").unwrap();
    let limits = InputLimits {
        max_bytes: usize::MAX,
    };
    assert_eq!(read_file_limited(&path, &limits).unwrap().len(), 10);
    std::fs::remove_dir_all(&dir).ok();
}

// ---- bug hunt (round 4): words that are not numbers, formats with absurd digits, bytes of every shape ----

#[test]
fn the_words_for_infinity_and_not_a_number_are_invalid_and_not_too_large() {
    for word in [
        "inf",
        "-inf",
        "+inf",
        "INF",
        "Infinity",
        "-infinity",
        "nan",
        "NaN",
        "-nan",
    ] {
        assert_eq!(parse_number(word), Err(NumberError::Invalid), "{word:?}");
        assert_eq!(parse_f64(word), None, "{word:?}");
    }
}

#[test]
fn digits_that_overflow_a_double_are_too_large_with_either_sign() {
    let long = "9".repeat(400);
    for big in ["1e999", "-1e999", "1E400", long.as_str(), "-9e999999999"] {
        assert_eq!(parse_number(big), Err(NumberError::TooLarge), "{big:.20}");
    }
}

#[test]
fn the_limit_of_magnitude_is_exact_for_whole_numbers_on_both_sides() {
    assert_eq!(parse_number("1000000000000000"), Ok(1e15));
    assert_eq!(parse_number("-1000000000000000"), Ok(-1e15));
    assert_eq!(parse_number("1000000000000001"), Err(NumberError::TooLarge));
    assert_eq!(
        parse_number("-1000000000000001"),
        Err(NumberError::TooLarge)
    );
}

#[test]
fn a_number_too_small_for_a_double_is_zero_and_not_an_error() {
    assert_eq!(parse_number("1e-999"), Ok(0.0));
    assert_eq!(parse_number("-1e-999"), Ok(0.0));
    assert_eq!(parse_number("4.9e-324"), Ok(4.9e-324));
}

#[test]
fn a_number_with_a_stray_character_or_digits_of_another_script_is_invalid() {
    for bad in [
        "5 5",
        "5\u{0}",
        "\u{0}5",
        "\u{ff15}",
        "\u{665}",
        "5\u{200b}",
        "1,000",
        "1..2",
        "++5",
        "+-5",
        "5e+",
        "0b101",
        "1e1e1",
    ] {
        assert_eq!(parse_number(bad), Err(NumberError::Invalid), "{bad:?}");
    }
    assert_eq!(parse_number("\t5\n"), Ok(5.0));
}

#[test]
fn a_format_with_absurd_digits_is_refused_and_never_panics_or_scales_by_a_wrapped_power() {
    for z in [ZeroSuppression::Leading, ZeroSuppression::Trailing] {
        assert_eq!(decode_coordinate("1", u32::MAX, u32::MAX, z), None);
        assert_eq!(decode_coordinate("1", u32::MAX, 1, z), None);
        assert_eq!(decode_coordinate("1", 1, u32::MAX, z), None);
        assert_eq!(decode_coordinate("5", 0, u32::MAX, z), None);
        assert_eq!(decode_coordinate("5", 0, 4_294_967_295 - 3, z), None);
    }
}

#[test]
fn the_most_digits_a_format_may_have_is_exact() {
    use eh3d_parser_common::MAX_FORMAT_DIGITS;
    assert_eq!(MAX_FORMAT_DIGITS, 64);
    assert!(decode_coordinate("1", 32, 32, ZeroSuppression::Leading).is_some());
    assert!(decode_coordinate("1", 33, 32, ZeroSuppression::Leading).is_none());
    assert!(decode_coordinate("1", 64, 0, ZeroSuppression::Trailing).is_some());
    assert!(decode_coordinate("1", 65, 0, ZeroSuppression::Trailing).is_none());
}

#[test]
fn a_format_with_no_digits_takes_only_numbers_with_a_point() {
    for z in [ZeroSuppression::Leading, ZeroSuppression::Trailing] {
        assert_eq!(decode_coordinate("5", 0, 0, z), None);
        assert_eq!(decode_coordinate("0", 0, 0, z), None);
        assert_eq!(decode_coordinate("5.", 0, 0, z), Some(5.0));
        assert_eq!(decode_coordinate(".25", 0, 0, z), Some(0.25));
    }
}

#[test]
fn digits_of_other_scripts_are_not_a_coordinate() {
    for z in [ZeroSuppression::Leading, ZeroSuppression::Trailing] {
        assert_eq!(decode_coordinate("\u{661}\u{662}\u{663}", 3, 3, z), None);
        assert_eq!(decode_coordinate("\u{ff11}\u{ff12}", 3, 3, z), None);
        assert_eq!(decode_coordinate("1\u{0}2", 3, 3, z), None);
        assert_eq!(decode_coordinate(" 12", 3, 3, z), None);
        assert_eq!(decode_coordinate("12 ", 3, 3, z), None);
    }
}

#[test]
fn a_decimal_coordinate_never_gives_a_number_that_is_not_finite() {
    for z in [ZeroSuppression::Leading, ZeroSuppression::Trailing] {
        for text in [
            "inf.0", "nan.0", "1e5.0", "1.5e5", "1e999.5", ".", "-.", "+.",
        ] {
            if let Some(v) = decode_coordinate(text, 3, 3, z) {
                assert!(v.is_finite(), "{text:?} {v}");
            }
        }
        assert_eq!(decode_coordinate("1000000000000001.5", 3, 3, z), None);
    }
}

#[test]
fn the_error_code_of_a_coordinate_with_absurd_digits_does_not_panic() {
    assert_eq!(
        coordinate_error_code("1", u32::MAX, u32::MAX),
        codes::COORDINATE_INVALID
    );
    assert_eq!(
        coordinate_error_code("12345", 0, 0),
        codes::NUMBER_TOO_LARGE
    );
}

#[test]
fn an_empty_input_and_a_byte_order_mark_alone_are_empty_text_without_a_notice() {
    for bytes in [&b""[..], &[0xEF, 0xBB, 0xBF][..]] {
        let d = decode_input(bytes, &InputLimits::default()).unwrap();
        assert_eq!(d.text, "");
        assert!(d.diagnostics.is_empty());
    }
}

#[test]
fn an_input_of_exactly_the_limit_is_read_and_one_byte_more_is_refused() {
    let limits = InputLimits { max_bytes: 4 };
    assert!(decode_input(b"abcd", &limits).is_ok());
    assert_eq!(
        decode_input(b"abcde", &limits),
        Err(InputError::TooLarge { size: 5, limit: 4 })
    );
    let none = InputLimits { max_bytes: 0 };
    assert!(decode_input(b"", &none).is_ok());
    assert!(decode_input(b"a", &none).is_err());
}

#[test]
fn bytes_that_are_not_utf8_become_the_characters_of_the_same_number_with_one_notice() {
    let bytes = [0xE9u8, 0xFF, 0x80, b'a'];
    let d = decode_input(&bytes, &InputLimits::default()).unwrap();
    assert_eq!(d.diagnostics.len(), 1);
    let back: Vec<u8> = d.text.chars().map(|c| c as u32 as u8).collect();
    assert_eq!(back, bytes);
    assert_eq!(d.text.chars().count(), 4);
}

#[test]
fn utf8_with_a_mark_loses_only_the_mark_and_keeps_every_character() {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice("G04 \u{e9}\u{65e5}\u{1f600}*\r\n".as_bytes());
    let d = decode_input(&bytes, &InputLimits::default()).unwrap();
    assert_eq!(d.text, "G04 \u{e9}\u{65e5}\u{1f600}*\r\n");
    assert!(d.diagnostics.is_empty());
}

#[test]
fn a_mark_cut_in_the_middle_is_read_as_latin_one_and_never_panics() {
    for bytes in [
        &[0xEFu8][..],
        &[0xEF, 0xBB][..],
        &[0xEF, 0xBB, 0xBF, 0xFF][..],
    ] {
        let d = decode_input(bytes, &InputLimits::default()).unwrap();
        assert!(!d.diagnostics.is_empty());
    }
}

#[test]
fn the_length_in_inches_is_converted_exactly_and_zero_and_signs_survive() {
    assert_eq!(Units::Inches.to_mm(1.0), 25.4);
    assert_eq!(Units::Inches.to_mm(-2.0), -50.8);
    assert_eq!(Units::Millimetres.to_mm(7.25), 7.25);
    assert_eq!(Units::Inches.to_mm(0.0), 0.0);
    assert!(Units::Inches.to_mm(f64::NAN).is_nan());
    assert!(Units::Inches.to_mm(f64::INFINITY).is_infinite());
}

#[test]
fn every_unit_less_file_adds_its_own_warning() {
    let mut d: Vec<Diagnostic> = Vec::new();
    assert_eq!(resolve_units(None, 1, &mut d), ASSUMED_UNITS);
    assert_eq!(resolve_units(None, 9, &mut d), ASSUMED_UNITS);
    assert_eq!(d.len(), 2);
    assert!(d.iter().all(|x| matches!(x.severity, Severity::Warning)));
    assert_eq!(d[1].line, 9);
}
