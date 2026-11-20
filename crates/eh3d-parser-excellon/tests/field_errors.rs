//! Regression tests of faults that other programs of the same kind had in the field, for the drill file reader: the
//! decimal comma of the language of the computer, settings kept from one file to the next, a reading that does not
//! end, a size of zero that ends in an exception, and precision far from the origin.

use eh3d_parser_excellon::{codes, parse, parse_with_limits, ExcellonLimits, Units};

/// The decimal comma: the same file with commas gives the same holes and tools, and says that it read commas.
#[test]
fn a_decimal_comma_gives_the_same_holes_as_a_decimal_point() {
    let point = "M48\nMETRIC\nT1C0.8\nT2C1.25\n%\nT1\nX12.5Y3.25\nT2\nX0.5Y100.125\nM30\n";
    let comma = "M48\nMETRIC\nT1C0,8\nT2C1,25\n%\nT1\nX12,5Y3,25\nT2\nX0,5Y100,125\nM30\n";
    let (a, b) = (parse(point), parse(comma));
    assert_eq!(a.tools, b.tools);
    let at = |f: &eh3d_parser_excellon::ExcellonFile| {
        f.drills.iter().map(|h| (h.tool, h.at)).collect::<Vec<_>>()
    };
    assert_eq!(at(&a), at(&b));
    assert!(!a.diagnostics.iter().any(|d| d.code == codes::DECIMAL_COMMA));
    assert!(b.diagnostics.iter().any(|d| d.code == codes::DECIMAL_COMMA));
}

/// Settings of the previous job leaked into the next: units, zeros, the format, the incremental mode, the tool in
/// the spindle and the route that was open. Nothing is kept between two readings.
#[test]
fn nothing_of_a_file_is_kept_for_the_next_one() {
    let messy = "M48\nINCH,LZ\nICI,ON\n; FILE_FORMAT=3:5\nT1C0.05\n%\nG91\nT1\nX010000Y010000\nG00X1Y1\nM15\nG01X2Y2\n";
    let clean = "M48\nMETRIC\nT1C0.8\n%\nT1\nX10.0Y10.0\nX20.0Y20.0\nM30\n";
    let alone = parse(clean);
    for _ in 0..3 {
        let _ = parse(messy);
        let again = parse(clean);
        assert_eq!(again, alone);
        assert_eq!(again.units, Some(Units::Millimetres));
        assert_eq!(again.drills[1].at.x, 20.0, "absolute, not incremental");
    }
    assert_eq!(parse(messy), parse(messy));
}

/// A reading that does not end: a file of millions of lines, a repeat of billions of holes. The work is bounded.
#[test]
fn a_hostile_file_ends_with_a_named_reason_and_what_was_read() {
    let mut text = String::from("M48\nMETRIC\nT1C0.8\n%\nT1\n");
    for i in 0..200_000 {
        text.push_str(&format!("X{}.0Y1.0\n", i % 1000));
    }
    text.push_str("M30\n");
    assert_eq!(parse(&text).drills.len(), 200_000);
    let cut = parse_with_limits(
        &text,
        &ExcellonLimits {
            max_items: 500,
            ..ExcellonLimits::default()
        },
    );
    assert_eq!(cut.drills.len(), 500);
    assert!(cut
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
    let repeat = parse("M48\nMETRIC\nT1C0.8\n%\nT1\nX1.0Y1.0\nR4000000000X0.001\nM30\n");
    assert_eq!(repeat.drills.len(), 1);
    assert!(repeat
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
}

/// A size of zero ended in an exception: a tool of no diameter, a hole at the origin, a route of one point and an
/// arc of no radius are all clean answers.
#[test]
fn a_size_of_zero_is_a_clean_answer_and_never_a_panic() {
    let tool = parse("M48\nMETRIC\nT1C0\nT2C0.8\n%\nT2\nX0Y0\nM30\n");
    assert!(tool
        .diagnostics
        .iter()
        .any(|d| d.code == codes::TOOL_DIAMETER_INVALID));
    assert!(!tool.tools.contains_key(&1));
    assert_eq!(tool.drills.len(), 1);
    assert_eq!((tool.drills[0].at.x, tool.drills[0].at.y), (0.0, 0.0));
    let route =
        parse("M48\nMETRIC\nT1C0.8\n%\nT1\nG00X0Y0\nM15\nM16\nG00X0Y0\nM15\nG02X0Y0A0\nM16\nM30\n");
    assert!(route.routes.iter().all(|r| !r.points.is_empty()));
    let empty = parse("");
    assert!(empty.drills.is_empty() && empty.routes.is_empty());
}

/// Precision far from the centre: the largest values a format holds are read to the last digit.
#[test]
fn the_precision_does_not_depend_on_the_distance_from_the_centre() {
    let f = parse("M48\nMETRIC\n; FILE_FORMAT=6:6\nT1C0.8\n%\nT1\nX999999.999999Y-999999.999999\nX0.000001Y0.000001\nM30\n");
    assert_eq!(f.drills[0].at.x, 999_999.999_999);
    assert_eq!(f.drills[0].at.y, -999_999.999_999);
    assert_eq!(f.drills[1].at.x, 0.000_001);
    // the same number as integers with the format 6:6
    let g = parse(
        "M48\nMETRIC,TZ\n; FILE_FORMAT=6:6\nT1C0.8\n%\nT1\nX999999999999Y-999999999999\nM30\n",
    );
    assert!((g.drills[0].at.x - 999_999.999_999).abs() < 1e-9);
    assert!((g.drills[0].at.y + 999_999.999_999).abs() < 1e-9);
}
