//! The reader must follow the modal state of a real program and survive damaged ones.

use eh3d_parser_gcode::{interpret, parse, Dialect, Motion, Severity};

fn run(text: &str) -> eh3d_parser_gcode::Trace {
    interpret(&parse(text, Dialect::Grbl))
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_small_milling_program_is_followed() {
    let t = run("G21 G90\nG0 Z5\nG0 X10 Y10\nG1 Z-0.1 F200\nG1 X20\nG1 Y20\nG0 Z5\nM30\n");
    assert!(t.diagnostics.is_empty(), "{:?}", t.diagnostics);
    assert!(t.ended);
    assert_eq!(t.segments.len(), 6);
    assert_eq!(t.segments[2].motion, Motion::Linear);
    assert_eq!(t.segments[2].feed, 200.0);
    assert_eq!((t.max.x, t.max.y, t.max.z), (20.0, 20.0, 5.0));
    assert_eq!(t.min.z, -0.1);
    assert!(
        close(t.cutting_length_mm, 5.1 + 10.0 + 10.0),
        "{}",
        t.cutting_length_mm
    );
}

#[test]
fn inches_become_millimetres_including_the_feed() {
    let t = run("G20\nG1 X1 F10\n");
    assert!(close(t.segments[0].to.x, 25.4));
    assert!(close(t.segments[0].feed, 254.0));
}

#[test]
fn relative_mode_adds_to_the_position() {
    let t = run("G90 G0 X10 Y10\nG91\nG1 X5 Y-2 F100\nG1 X5\n");
    assert_eq!((t.segments[1].to.x, t.segments[1].to.y), (15.0, 8.0));
    assert_eq!(t.segments[2].to.x, 20.0);
}

#[test]
fn modal_motion_and_feed_are_kept_between_lines() {
    let t = run("G1 X1 F300\nX2\nY3\n");
    assert_eq!(t.segments.len(), 3);
    assert!(t
        .segments
        .iter()
        .all(|s| s.motion == Motion::Linear && s.feed == 300.0));
}

#[test]
fn spindle_and_tool_are_tracked() {
    let t = run("T2 M6\nS12000 M3\nG1 X1 F100\nM5\nG1 X2\n");
    assert_eq!(t.segments[0].tool, Some(2));
    assert!(t.segments[0].spindle_on && t.segments[0].spindle_speed == 12000.0);
    assert!(!t.segments[1].spindle_on);
}

#[test]
fn an_arc_with_a_centre_has_the_right_length() {
    // a half circle of radius 5 from (0,0) to (10,0), centre at (5,0)
    let t = run("G90 G0 X0 Y0\nG2 X10 Y0 I5 J0 F100\n");
    assert!(
        matches!(t.segments[1].motion, Motion::ArcCw { center } if close(center.0, 5.0) && close(center.1, 0.0))
    );
    assert!(
        close(t.cutting_length_mm, std::f64::consts::PI * 5.0),
        "{}",
        t.cutting_length_mm
    );
    let t = run("G90 G0 X0 Y0\nG3 X10 Y0 I5 J0 F100\n");
    assert!(close(t.cutting_length_mm, std::f64::consts::PI * 5.0));
}

#[test]
fn a_full_circle_is_a_full_circle() {
    let t = run("G0 X5 Y0\nG2 X5 Y0 I-5 J0 F100\n");
    assert!(close(t.cutting_length_mm, std::f64::consts::TAU * 5.0));
}

#[test]
fn an_arc_with_a_radius_finds_its_centre() {
    // quarter circle clockwise from (5,0) to (0,-5) around the origin
    let t = run("G0 X5 Y0\nG2 X0 Y-5 R5 F100\n");
    let Motion::ArcCw { center } = t.segments[1].motion else {
        panic!("not an arc: {:?}", t.segments[1].motion)
    };
    assert!(close(center.0, 0.0) && close(center.1, 0.0), "{center:?}");
    assert!(close(t.cutting_length_mm, std::f64::consts::PI * 5.0 / 2.0));
    // the same chord counter-clockwise has its centre on the other side
    let t = run("G0 X5 Y0\nG3 X0 Y-5 R5 F100\n");
    let Motion::ArcCcw { center } = t.segments[1].motion else {
        panic!()
    };
    assert!(close(center.0, 5.0) && close(center.1, -5.0), "{center:?}");
}

#[test]
fn an_arc_that_cannot_exist_is_reported_and_drawn_straight() {
    let t = run("G0 X0 Y0\nG2 X100 Y0 R5 F100\nG2 X1 Y1 F100\n");
    assert_eq!(
        t.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count(),
        2
    );
    assert!(t
        .segments
        .iter()
        .skip(1)
        .all(|s| s.motion == Motion::Linear));
}

#[test]
fn comments_and_line_numbers_are_understood() {
    let p = parse(
        "N10 G1 X1 F100 (cut the edge) ; and a note\n; only a comment\n\n(another)\nG0 X0\n",
        Dialect::Grbl,
    );
    assert_eq!(p.blocks.len(), 2);
    assert_eq!(p.blocks[0].get('N'), Some(10.0));
    assert_eq!(
        p.blocks[0].comment.as_deref(),
        Some("cut the edge and a note")
    );
    assert_eq!(p.blocks[1].line, 5);
}

#[test]
fn marlin_checksums_are_verified() {
    let body = "N1 G1 X10 F100";
    let sum = body.bytes().fold(0u8, |a, b| a ^ b);
    let good = parse(&format!("{body}*{sum}\n"), Dialect::Marlin);
    assert!(good.is_clean(), "{:?}", good.diagnostics);
    let bad = parse(
        &format!("{body}*{}\n", sum.wrapping_add(1)),
        Dialect::Marlin,
    );
    assert!(!bad.is_clean());
    // in GRBL a star is not a checksum
    assert!(parse("G1 X10 F100\n", Dialect::Grbl).is_clean());
}

#[test]
fn grbl_system_commands_are_skipped_with_a_note() {
    let p = parse("$H\n$$\nG1 X1 F10\n", Dialect::Grbl);
    assert_eq!(p.blocks.len(), 1);
    assert_eq!(
        p.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Info)
            .count(),
        2
    );
}

#[test]
fn unsupported_features_are_errors_not_guesses() {
    let t = run("G41 D1\nG81 Z-2 R2\nG1 X1 F10\n");
    assert_eq!(
        t.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count(),
        2
    );
    assert_eq!(t.segments.len(), 1);
}

#[test]
fn an_unknown_code_is_a_warning_and_a_bad_number_an_error() {
    let t = run("G999\nM77\nG1 X1.2.3 F10\nG1 Xabc\n");
    assert!(t
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("G999")));
    assert!(t
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("M77")));
    assert!(
        t.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
            >= 2
    );
}

#[test]
fn a_move_without_a_feed_and_text_after_the_end_are_reported() {
    let t = run("G1 X1\nM2\nG1 X5 F10\n");
    assert!(t.diagnostics.iter().any(|d| d.message.contains("no feed")));
    assert!(t
        .diagnostics
        .iter()
        .any(|d| d.message.contains("after M2/M30")));
    assert_eq!(t.segments.len(), 1);
}

#[test]
fn an_axis_word_before_any_motion_code_is_a_rapid_with_a_warning() {
    let t = run("X5 Y5\n");
    assert_eq!(t.segments[0].motion, Motion::Rapid);
    assert!(t
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning));
}

#[test]
fn long_lines_and_garbage_never_panic() {
    let long = format!("G1 X1 {}\n", "(".repeat(5000));
    let _ = interpret(&parse(&long, Dialect::Grbl));
    for text in [
        "",
        "\u{0}\u{1}",
        "G",
        "G1 X",
        "X1e999",
        "G1 X1 Y\n(((\n)))",
        "T99999999999 M6",
        "F-1 S-1",
        "G91.1 G90.1\nG2 X1 Y1 I0 J0",
        "\u{feff}G1 X1 F1",
    ] {
        for dialect in [Dialect::Grbl, Dialect::Marlin, Dialect::PlanetCnc] {
            let _ = interpret(&parse(text, dialect));
        }
    }
    let p = parse(&long, Dialect::Grbl);
    assert!(p
        .diagnostics
        .iter()
        .any(|d| d.message.contains("longer than")));
}

#[test]
fn absolute_arc_centres_are_honoured() {
    let t = run("G90.1\nG0 X0 Y0\nG2 X10 Y0 I5 J0 F100\n");
    let Motion::ArcCw { center } = t.segments[1].motion else {
        panic!()
    };
    assert!(close(center.0, 5.0) && close(center.1, 0.0));
}
