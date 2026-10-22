//! The reader must understand real drill files and survive damaged ones.

use eh3d_parser_excellon::{decode_coordinate, parse, Severity, Units, ZeroSuppression};

const KICAD: &str = "M48\n; FILE_FORMAT=4:3\nMETRIC,TZ\nT1C0.800\nT2C1.000\n%\nG90\nG05\nT1\nX10.5Y20.25\nX11.5Y21\nT2\nX0Y0\nM30\n";

#[test]
fn a_typical_metric_file_is_read() {
    let f = parse(KICAD);
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    assert_eq!(f.units, Some(Units::Millimetres));
    assert_eq!(f.zeros, Some(ZeroSuppression::Leading));
    assert_eq!(f.tools.len(), 2);
    assert_eq!(f.tools[&1].diameter, 0.8);
    assert_eq!(f.drills.len(), 3);
    assert_eq!((f.drills[0].at.x, f.drills[0].at.y), (10.5, 20.25));
    assert_eq!(f.drills[2].tool, 2);
    assert!(f.ended);
}

#[test]
fn a_missing_coordinate_repeats_the_previous_one() {
    let f = parse("M48\nMETRIC\nT1C1\n%\nT1\nX1.0Y2.0\nX3.0\nY4.0\nM30\n");
    let p: Vec<_> = f.drills.iter().map(|h| (h.at.x, h.at.y)).collect();
    assert_eq!(p, [(1.0, 2.0), (3.0, 2.0), (3.0, 4.0)]);
}

#[test]
fn slots_with_g85_are_read() {
    let f = parse("M48\nMETRIC\nT1C0.6\n%\nT1\nX1.0Y1.0G85X5.0Y1.0\nM30\n");
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    assert_eq!(f.slots.len(), 1);
    assert_eq!((f.slots[0].from.x, f.slots[0].to.x), (1.0, 5.0));
    assert!(f.drills.is_empty());
}

#[test]
fn a_route_collects_its_points() {
    let f = parse("M48\nMETRIC\nT1C2\n%\nT1\nG00X0Y0\nM15\nG01X10Y0\nX10Y10\nM16\nM30\n");
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    assert_eq!(f.routes.len(), 1);
    assert_eq!(f.routes[0].points.len(), 3);
    assert!(f.drills.is_empty());
}

#[test]
fn inch_files_convert_to_millimetres() {
    let f = parse("M48\nINCH,LZ\nT1C0.0315\n%\nT1\nX010000Y020000\nM30\n");
    assert_eq!(f.units, Some(Units::Inches));
    assert_eq!(f.zeros, Some(ZeroSuppression::Trailing));
    assert!((Units::Inches.to_mm(1.0) - 25.4).abs() < 1e-12);
    assert_eq!((f.drills[0].at.x, f.drills[0].at.y), (1.0, 2.0));
}

#[test]
fn coordinates_follow_the_zero_suppression() {
    assert_eq!(
        decode_coordinate("1500", 3, 3, ZeroSuppression::Leading),
        Some(1.5)
    );
    assert_eq!(
        decode_coordinate("15", 3, 3, ZeroSuppression::Trailing),
        Some(150.0)
    );
    assert_eq!(
        decode_coordinate("-2.5", 3, 3, ZeroSuppression::Leading),
        Some(-2.5)
    );
    assert_eq!(
        decode_coordinate("1234567", 3, 3, ZeroSuppression::Leading),
        None
    );
    assert_eq!(
        decode_coordinate("12a", 3, 3, ZeroSuppression::Leading),
        None
    );
    assert_eq!(decode_coordinate("", 3, 3, ZeroSuppression::Leading), None);
}

#[test]
fn problems_become_diagnostics_and_reading_goes_on() {
    let f = parse("M48\nMETRIC\nT1C0.8\nT2\n%\nT1\nXabcY1\nX1.0Y1.0\nT9\nX2.0Y2.0\nM30\n");
    assert!(!f.is_clean());
    let errors: Vec<_> = f
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert!(errors.len() >= 3, "{errors:?}");
    assert!(errors.iter().all(|d| d.line >= 4));
    assert!(f.drills.iter().any(|h| h.at.x == 1.0));
}

#[test]
fn a_hole_before_any_tool_is_an_error() {
    let f = parse("M48\nMETRIC\n%\nX1Y1\nM30\n");
    assert!(f.drills.is_empty());
    assert!(!f.is_clean());
}

#[test]
fn a_file_without_m30_or_header_warns_but_is_clean() {
    let f = parse("T1\n");
    assert!(f
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning));
    let g = parse("M48\nMETRIC\nT1C1\n%\nT1\nX1Y1\n");
    assert!(g.is_clean());
    assert!(!g.ended);
}

#[test]
fn unsupported_features_are_reported_not_guessed() {
    let f = parse("M48\nMETRIC\nT1C1\n%\nT1\nG91\nG02X1Y1\nM30\n");
    assert!(
        f.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
            >= 2
    );
}

#[test]
fn garbage_never_panics() {
    for text in [
        "",
        "\u{0}\u{1}\u{2}",
        "M48\n%\nT\nTC\nT1C-1\nXY\nG85\nM15\nM16\nM30\nM30\n",
        "X\nY\nG\nM\n",
        "T99999999999999999999C1",
    ] {
        let _ = parse(text);
    }
}
