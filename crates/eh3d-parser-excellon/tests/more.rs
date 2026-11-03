//! More of the Excellon reader: signs, decimals, repeated tools, units and line ends.

use eh3d_parser_excellon::{codes, parse, parse_bytes, Units, ZeroSuppression};

fn holes(text: &str) -> Vec<(u32, f64, f64)> {
    parse(text)
        .drills
        .iter()
        .map(|h| (h.tool, h.at.x, h.at.y))
        .collect()
}

#[test]
fn negative_and_decimal_coordinates_are_read_as_written() {
    let h = holes("M48\nMETRIC\nT1C0.8\n%\nT1\nX-1.5Y2.25\nX.5Y-.5\nM30\n");
    assert_eq!(h, [(1, -1.5, 2.25), (1, 0.5, -0.5)]);
}

#[test]
fn a_tool_selected_twice_collects_all_its_holes_and_the_diameter_stays() {
    let f =
        parse("M48\nMETRIC\nT1C0.8\nT2C1.2\n%\nT1\nX1.0Y1.0\nT2\nX2.0Y2.0\nT1\nX3.0Y3.0\nM30\n");
    let by_tool = |t: u32| f.drills.iter().filter(|h| h.tool == t).count();
    assert_eq!((by_tool(1), by_tool(2)), (2, 1));
    assert_eq!((f.tools[&1].diameter, f.tools[&2].diameter), (0.8, 1.2));
}

#[test]
fn windows_line_ends_and_a_byte_order_mark_change_nothing() {
    let lf = "M48\nMETRIC,TZ\nT1C0.8\n%\nT1\nX1.0Y1.0\nX2.0Y2.0\nM30\n";
    let want = parse(lf);
    assert_eq!(parse(&lf.replace('\n', "\r\n")).drills, want.drills);
    let mut marked = vec![0xEF, 0xBB, 0xBF];
    marked.extend_from_slice(lf.as_bytes());
    let got = parse_bytes(&marked).unwrap();
    assert_eq!(got.drills, want.drills);
    assert!(got.is_clean(), "{:?}", got.diagnostics);
}

#[test]
fn the_default_metric_format_applies_when_the_header_gives_none() {
    let f = parse("M48\nMETRIC\nT1C0.8\n%\nT1\nX010500Y020250\nM30\n");
    assert_eq!(f.units, Some(Units::Millimetres));
    assert_eq!(
        (f.drills[0].at.x, f.drills[0].at.y),
        (10.5, 20.25),
        "metric with leading zeros kept is 3.3: 010500 is 10.500"
    );
}

#[test]
fn inch_trailing_zero_suppression_pads_on_the_right() {
    let f = parse("M48\nINCH,LZ\nT1C0.03\n%\nT1\nX1Y15\nM30\n");
    assert_eq!(f.zeros, Some(ZeroSuppression::Trailing));
    assert_eq!(
        (f.drills[0].at.x, f.drills[0].at.y),
        (10.0, 15.0),
        "with the trailing zeros left out X1 is 10.0000 and Y15 is 15.0000 in the 2.4 inch format"
    );
}

#[test]
fn a_file_with_no_holes_is_clean_and_empty() {
    let f = parse("M48\nMETRIC\nT1C0.8\n%\nM30\n");
    assert!(f.drills.is_empty() && f.slots.is_empty() && f.routes.is_empty());
    assert!(f.ended);
    assert!(parse("").drills.is_empty());
    assert!(parse("\n\n\n").drills.is_empty());
}

#[test]
fn a_tool_that_was_never_defined_is_reported_and_the_holes_after_it_are_still_read() {
    let f = parse("M48\nMETRIC\nT1C0.8\n%\nT9\nX1.0Y1.0\nT1\nX2.0Y2.0\nM30\n");
    assert!(!f.is_clean(), "{:?}", f.diagnostics);
    assert!(f.drills.iter().any(|h| h.tool == 1 && h.at.x == 2.0));
}

#[test]
fn the_same_text_read_twice_is_the_same_file() {
    let text = "M48\nMETRIC\nT1C0.8\n%\nT1\nX1.0Y1.0G85X5.0Y1.0\nX2.0Y2.0\nM30\n";
    assert_eq!(parse(text), parse(text));
}

#[test]
fn a_file_with_no_units_whose_holes_are_tiny_may_be_in_inches() {
    let text = "M48\nT1C0.03\n%\nT1\nX0.5Y0.5\nX2.5Y1.5\nM30\n";
    let f = parse(text);
    assert!(f.diagnostics.iter().any(|d| d.code == codes::UNITS_SUSPECT));
    // a metric file of the same holes, a larger board and a single hole are not suspect
    for other in [
        "METRIC\nT1C0.8\n%\nT1\nX0.5Y0.5\nX2.5Y1.5\nM30\n",
        "M48\nT1C0.8\n%\nT1\nX10Y10\nX60Y40\nM30\n",
        "M48\nT1C0.8\n%\nT1\nX1Y1\nM30\n",
    ] {
        let f = parse(other);
        assert!(
            f.diagnostics.iter().all(|d| d.code != codes::UNITS_SUSPECT),
            "{other:?}"
        );
    }
}
