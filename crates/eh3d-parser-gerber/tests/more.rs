//! More of the Gerber reader: signs, arcs, polarity, line ends, several commands in one line and the units.

use eh3d_parser_gerber::{
    parse, parse_bytes, Aperture, Operation, Point, Polarity, QuadrantMode, Units,
};

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}

const HEAD: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\n";

fn ops(text: &str) -> Vec<Operation> {
    parse(text).commands.into_iter().map(|c| c.op).collect()
}

#[test]
fn negative_coordinates_keep_their_sign_and_scale() {
    let o = ops(&format!(
        "{HEAD}X-50000Y-100000D02*\nX+25000Y-1D01*\nM02*\n"
    ));
    assert_eq!(o[1], Operation::Move { to: p(-5.0, -10.0) });
    assert_eq!(
        o[2],
        Operation::Line {
            from: p(-5.0, -10.0),
            to: p(2.5, -0.0001)
        }
    );
}

#[test]
fn a_coordinate_with_a_decimal_point_is_taken_as_it_is_whatever_the_format() {
    let o = ops(&format!("{HEAD}X12.5Y-3.25D03*\nM02*\n"));
    assert_eq!(o[1], Operation::Flash { at: p(12.5, -3.25) });
}

#[test]
fn a_counterclockwise_arc_in_single_quadrant_mode_is_read_with_both_marks() {
    let o = ops(&format!(
        "{HEAD}G74*\nX0Y0D02*\nG03*\nX10000Y10000I10000J0D01*\nM02*\n"
    ));
    let arc = o
        .iter()
        .find(|op| matches!(op, Operation::Arc { .. }))
        .expect("an arc");
    assert_eq!(
        arc,
        &Operation::Arc {
            from: p(0.0, 0.0),
            to: p(1.0, 1.0),
            center_offset: (1.0, 0.0),
            clockwise: false,
            quadrant: QuadrantMode::Single
        }
    );
}

#[test]
fn the_polarity_commands_come_out_in_order_and_dark_is_the_default() {
    let o = ops(&format!("{HEAD}%LPC*%\nX1Y1D03*\n%LPD*%\nX2Y2D03*\nM02*\n"));
    let pol: Vec<_> = o
        .iter()
        .filter_map(|op| {
            if let Operation::Polarity(x) = op {
                Some(*x)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(pol, [Polarity::Clear, Polarity::Dark]);
}

#[test]
fn windows_line_ends_give_the_same_commands_and_the_same_lines() {
    let lf = format!("{HEAD}X10000Y10000D02*\nX20000Y10000D01*\nX30000Y30000D03*\nM02*\n");
    let crlf = lf.replace('\n', "\r\n");
    let (a, b) = (parse(&lf), parse(&crlf));
    assert_eq!(a.commands, b.commands);
    assert_eq!(a.diagnostics, b.diagnostics);
    assert!(a.ended && b.ended);
}

#[test]
fn several_blocks_on_one_line_are_read_in_order_and_keep_that_line() {
    let file = parse(&format!(
        "{HEAD}X10000Y10000D02*X20000Y10000D01*X30000Y30000D03*M02*\n"
    ));
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let n = file.commands.len();
    assert!(n >= 3);
    let last_three: Vec<_> = file.commands[n - 3..].iter().map(|c| c.line).collect();
    assert_eq!(last_three, [5, 5, 5]);
    assert!(file.ended);
}

#[test]
fn inch_files_are_read_as_inches_and_the_numbers_are_not_converted_by_the_reader() {
    let file = parse("%FSLAX24Y24*%\n%MOIN*%\n%ADD10C,0.01*%\nD10*\nX10000Y20000D03*\nM02*\n");
    assert_eq!(file.units, Some(Units::Inches));
    assert_eq!(file.effective_units(), Units::Inches);
    assert_eq!(file.commands[1].op, Operation::Flash { at: p(1.0, 2.0) });
    assert_eq!(
        file.apertures[&10],
        Aperture::Circle {
            diameter: 0.01,
            hole: None
        }
    );
}

#[test]
fn an_empty_file_and_a_file_with_only_comments_are_read_without_commands() {
    for text in [
        "",
        "\n\n",
        "G04 nothing here*\nM02*\n",
        "\u{feff}%FSLAX24Y24*%\n%MOMM*%\nM02*\n",
    ] {
        let file = parse(text);
        assert!(file.commands.is_empty(), "{text:?}");
    }
    assert!(
        parse("\u{feff}%FSLAX24Y24*%\n%MOMM*%\nM02*\n").ended,
        "a byte order mark in front of the text is skipped"
    );
}

#[test]
fn bytes_with_a_mark_and_text_that_is_not_utf8_give_the_same_commands_as_plain_text() {
    let text = format!("{HEAD}X10000Y10000D03*\nM02*\n");
    let plain = parse_bytes(text.as_bytes()).unwrap();
    let mut marked = vec![0xEF, 0xBB, 0xBF];
    marked.extend_from_slice(text.as_bytes());
    assert_eq!(parse_bytes(&marked).unwrap().commands, plain.commands);
    // a comment with a byte that is not UTF-8 does not change the geometry
    let mut latin = b"G04 caf".to_vec();
    latin.push(0xE9);
    latin.extend_from_slice(b"*\n");
    latin.extend_from_slice(text.as_bytes());
    assert_eq!(
        parse_bytes(&latin).unwrap().commands.len(),
        plain.commands.len()
    );
}

#[test]
fn flashing_with_no_aperture_chosen_is_reported_and_the_reader_goes_on() {
    let file = parse(
        "%FSLAX24Y24*%\n%MOMM*%\nX10000Y10000D03*\n%ADD10C,0.1*%\nD10*\nX20000Y20000D03*\nM02*\n",
    );
    assert!(!file.is_clean());
    assert!(
        file.commands
            .iter()
            .any(|c| c.op == Operation::Flash { at: p(2.0, 2.0) }),
        "what comes after the mistake is still read"
    );
}

#[test]
fn the_same_text_read_twice_is_the_same_file() {
    let text = format!("{HEAD}X10000Y10000D02*\nX20000Y10000D01*\nG36*\nX0Y0D02*\nX1000Y0D01*\nX1000Y1000D01*\nG37*\nM02*\n");
    assert_eq!(parse(&text), parse(&text));
}

#[test]
fn a_draw_with_no_start_point_does_not_make_the_reading_quadratic() {
    // 150000 flashless draws after 150000 aperture selections: looking back through the commands
    // for the last point on every draw took minutes
    let mut text = String::from("%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\n");
    for _ in 0..150_000 {
        text.push_str("D10*\n");
    }
    for _ in 0..150_000 {
        text.push_str("X1Y1D01*\n");
    }
    let started = std::time::Instant::now();
    let file = parse(&text);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "took {:?}",
        started.elapsed()
    );
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.code == eh3d_parser_gerber::codes::START_POINT_MISSING));
}

#[test]
fn the_start_point_is_the_end_of_the_last_move_flash_line_or_arc() {
    let o = ops(&format!(
        "{HEAD}X10000Y10000D03*\nX20000Y10000D01*\nX30000Y10000D02*\nX40000Y10000D01*\nM02*\n"
    ));
    assert_eq!(
        o[2],
        Operation::Line {
            from: p(1.0, 1.0),
            to: p(2.0, 1.0)
        }
    );
    assert_eq!(
        o[4],
        Operation::Line {
            from: p(3.0, 1.0),
            to: p(4.0, 1.0)
        }
    );
}

#[test]
fn the_diagnostics_kept_are_limited_and_the_cut_is_said_once() {
    let mut text = String::from("%FSLAX24Y24*%\n%MOMM*%\n");
    for _ in 0..(eh3d_parser_gerber::MAX_DIAGNOSTICS + 500) {
        text.push_str("X1Y1D01*\n");
    }
    let file = parse(&text);
    assert!(file.diagnostics.len() <= eh3d_parser_gerber::MAX_DIAGNOSTICS + 3);
    let cuts = file
        .diagnostics
        .iter()
        .filter(|d| d.code == eh3d_parser_gerber::codes::TOO_MANY_DIAGNOSTICS)
        .count();
    assert_eq!(cuts, 1);
}

#[test]
fn a_layer_transform_that_changes_the_drawing_is_an_error_and_one_that_changes_nothing_is_not() {
    use eh3d_parser_gerber::{codes, Severity};
    for cmd in ["LR90", "LR-45.5", "LMX", "LMXY", "LS0.5", "LS2", "AB10"] {
        let file = parse(&format!("{HEAD}%{cmd}*%\nM02*\n"));
        assert!(
            file.diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error
                    && d.code == codes::GEOMETRY_COMMAND_UNSUPPORTED),
            "{cmd}: {:?}",
            file.diagnostics
        );
        assert!(!file.is_clean(), "{cmd}");
    }
    for cmd in ["LR0", "LR0.0", "LMN", "LS1", "LS1.0"] {
        let file = parse(&format!("{HEAD}%{cmd}*%\nM02*\n"));
        assert!(file.is_clean(), "{cmd}: {:?}", file.diagnostics);
    }
}
