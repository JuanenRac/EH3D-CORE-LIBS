//! The one way the programs of the suite write a number and the header of a file.

use eh3d_parser_gcode::emit::{header, number, sha256_hex, Header};
use eh3d_parser_gcode::{interpret, parse, Dialect};

#[test]
fn a_number_is_short_has_no_negative_zero_and_is_never_nan_or_inf() {
    assert_eq!(number(12.0).unwrap(), "12");
    assert_eq!(number(-0.0004).unwrap(), "0");
    assert_eq!(number(1.2346).unwrap(), "1.235");
    // 1.2345 is stored a hair under that value, so it rounds down: the text is what the number really is
    assert_eq!(number(1.2345).unwrap(), "1.234");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(number(bad).is_err());
    }
}

#[test]
fn what_is_written_is_read_back_by_the_reader_of_the_same_crate() {
    let h = header(&Header {
        generator: "g",
        version: "1",
        source: Some(("in.gbr", sha256_hex(b"x"))),
        tools: "t",
    });
    let program = format!(
        "{h}G21\nG90\nG1 X{} Y{} F100\nM2\n",
        number(1.5).unwrap(),
        number(-2.0).unwrap()
    );
    let p = parse(&program, Dialect::Grbl);
    assert!(p.is_clean(), "{:?}", p.diagnostics);
    let t = interpret(&p);
    assert_eq!(t.segments.len(), 1);
    assert_eq!((t.segments[0].to.x, t.segments[0].to.y), (1.5, -2.0));
}

#[test]
fn a_name_cannot_put_a_real_time_command_of_grbl_in_the_header() {
    let h = header(&Header {
        generator: "g",
        version: "1",
        source: Some(("plate!.gtp job.nc~ cut?\u{18}\u{e9}\nM3", sha256_hex(b"x"))),
        tools: "t",
    });
    for bad in ['!', '~', '?', '\u{18}', '\u{e9}'] {
        assert!(!h.contains(bad), "{bad:?} in {h:?}");
    }
    assert!(
        h.contains("; source: plate_.gtp job.nc_ cut___ M3 sha256:"),
        "{h}"
    );
    assert_eq!(h.lines().count(), 4, "a name added a line: {h:?}");
}

// ---- bug hunt (round 4): the text of a number at its edges, the hash across the boundaries of its blocks, hostile names in the header ----

#[test]
fn a_number_is_written_with_at_most_three_decimals_and_without_zeros_that_say_nothing() {
    for (v, want) in [
        (100.0, "100"),
        (-100.0, "-100"),
        (0.1, "0.1"),
        (10.5, "10.5"),
        (0.999, "0.999"),
        (0.9999, "1"),
        (-0.9999, "-1"),
        (12.3454, "12.345"),
        (-1e-9, "0"),
        (1e-9, "0"),
        (0.0, "0"),
        (-0.0, "0"),
        (1e15, "1000000000000000"),
        (-7.0, "-7"),
        (20.0, "20"),
        (1000.0, "1000"),
    ] {
        assert_eq!(number(v).unwrap(), want, "{v}");
    }
}

#[test]
fn a_very_large_number_is_written_in_digits_and_never_with_an_exponent_or_a_word() {
    for v in [1e20, -1e20, 1e300, f64::MAX, f64::MIN, f64::MIN_POSITIVE] {
        let s = number(v).unwrap();
        assert!(
            s.chars().all(|c| c.is_ascii_digit() || c == '-'),
            "{v}: {s:.40}"
        );
    }
    assert_eq!(number(1e300).unwrap().len(), 301);
}

#[test]
fn a_number_reads_back_within_half_a_thousandth_and_writing_it_again_changes_nothing() {
    let mut v = -1000.0_f64;
    while v < 1000.0 {
        let s = number(v).unwrap();
        let back: f64 = s.parse().unwrap();
        assert!((back - v).abs() <= 0.0005 + 1e-9, "{v} {s}");
        assert_eq!(number(back).unwrap(), s, "{v}");
        let decimals = s.split_once('.').map_or(0, |(_, d)| d.len());
        assert!(decimals <= 3, "{s}");
        assert!(
            !s.contains('.') || !(s.ends_with('.') || s.ends_with('0')),
            "{s}"
        );
        assert!(s != "-0" && !s.is_empty(), "{v}");
        v += 0.0137;
    }
}

#[test]
fn the_hash_of_a_message_of_two_blocks_and_of_a_million_letters_is_the_one_of_the_standard() {
    assert_eq!(
        sha256_hex(
            b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"
        ),
        "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1"
    );
    assert_eq!(
        sha256_hex(&vec![b'a'; 1_000_000]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn the_hash_of_every_length_around_the_blocks_is_sixty_four_lowercase_digits_and_never_repeats() {
    let mut seen = std::collections::BTreeSet::new();
    for n in 0..=200usize {
        let data: Vec<u8> = (0..n).map(|i| (i * 7 + 3) as u8).collect();
        let h = sha256_hex(&data);
        assert_eq!(h.len(), 64, "{n}");
        assert!(
            h.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "{n} {h}"
        );
        assert!(seen.insert(h), "length {n} repeats a hash");
    }
}

#[test]
fn changing_one_bit_of_the_data_changes_the_hash_at_every_place() {
    let data = [0x5au8; 130];
    let base = sha256_hex(&data);
    for at in [0usize, 55, 56, 63, 64, 65, 119, 120, 129] {
        let mut copy = data;
        copy[at] ^= 1;
        assert_ne!(sha256_hex(&copy), base, "{at}");
    }
}

#[test]
fn a_header_with_hostile_names_stays_four_lines_of_comments() {
    let h = header(&Header {
        generator: "gen\nG1 X5\r\nM3 S1000\n!~?",
        version: "\u{e9}\u{202e}\u{0}\u{7f}\u{1f600}",
        source: Some(("f\nM3.gbr\r\nG0 X0", sha256_hex(b"x"))),
        tools: "t\u{0}\n\r\u{85}\u{2028}M5",
    });
    assert_eq!(h.lines().count(), 4, "{h:?}");
    assert!(h.lines().all(|l| l.starts_with("; ")), "{h:?}");
    assert!(
        h.chars()
            .all(|c| c == '\n' || c.is_ascii_graphic() || c == ' '),
        "{h:?}"
    );
}

#[test]
fn a_header_without_a_source_says_none_and_a_header_with_units_keeps_them() {
    let h = header(&Header {
        generator: "g",
        version: "1",
        source: None,
        tools: "t",
    });
    assert!(h.contains("; source: none\n"), "{h}");
    let inch = eh3d_parser_gcode::emit::header_with_units(
        &Header {
            generator: "g",
            version: "1",
            source: None,
            tools: "t",
        },
        "inch (G20)\nG1",
    );
    assert!(inch.contains("; units: inch (G20) G1\n"), "{inch}");
    assert_eq!(inch.lines().count(), 4);
}
