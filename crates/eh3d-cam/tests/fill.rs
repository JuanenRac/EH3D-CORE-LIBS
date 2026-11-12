use eh3d_cam::fill::{rubout_contours, Fill};
use eh3d_cam::transform::Axis;
use eh3d_geometry::scene::{Scene, Shape};

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<(f64, f64)> {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn fill(tool: f64, overlap: f64, direction: Axis) -> Fill {
    Fill {
        tool_mm: tool,
        overlap,
        direction,
        margin_mm: 0.0,
        res_mm: 0.05,
    }
}

#[test]
fn a_square_is_filled_with_lines_one_tool_apart_that_go_back_and_forth() {
    let pass = rubout_contours(
        &[square(0.0, 0.0, 10.0, 10.0)],
        None,
        &fill(1.0, 0.0, Axis::Horizontal),
    )
    .unwrap();
    assert!(pass.rubout && pass.open.len() == 10 && pass.paths.is_empty());
    assert_eq!(pass.diameter_mm, 1.0);
    for (k, run) in pass.open.iter().enumerate() {
        assert_eq!(run.len(), 2);
        // the centre of the tool is half a tool from the edge on every side
        let y = 0.5 + k as f64;
        assert!(
            (run[0].1 - y).abs() < 1e-9 && (run[1].1 - y).abs() < 1e-9,
            "{run:?}"
        );
        let (from, to) = if k % 2 == 0 { (0.5, 9.5) } else { (9.5, 0.5) };
        assert!(
            (run[0].0 - from).abs() < 1e-9 && (run[1].0 - to).abs() < 1e-9,
            "{k}: {run:?}"
        );
    }
}

#[test]
fn an_overlap_makes_the_lines_closer() {
    let sq = [square(0.0, 0.0, 10.0, 10.0)];
    let none = rubout_contours(&sq, None, &fill(1.0, 0.0, Axis::Horizontal)).unwrap();
    let half = rubout_contours(&sq, None, &fill(1.0, 0.5, Axis::Horizontal)).unwrap();
    assert_eq!(none.open.len(), 10);
    assert_eq!(half.open.len(), 19);
}

#[test]
fn vertical_lines_trade_the_coordinates() {
    let pass = rubout_contours(
        &[square(0.0, 0.0, 10.0, 10.0)],
        None,
        &fill(1.0, 0.0, Axis::Vertical),
    )
    .unwrap();
    assert_eq!(pass.open.len(), 10);
    for (k, run) in pass.open.iter().enumerate() {
        assert!(
            (run[0].0 - (0.5 + k as f64)).abs() < 1e-9 && (run[1].0 - run[0].0).abs() < 1e-9,
            "{run:?}"
        );
        assert!((run[0].1 - run[1].1).abs() > 8.0);
    }
}

#[test]
fn a_contour_inside_another_is_a_hole_that_the_lines_go_round() {
    let contours = [square(0.0, 0.0, 10.0, 10.0), square(3.0, 3.0, 7.0, 7.0)];
    let pass = rubout_contours(&contours, None, &fill(1.0, 0.0, Axis::Horizontal)).unwrap();
    // the line at y = 4.5 crosses the hole: two runs, one each side of it
    let at: Vec<&Vec<(f64, f64)>> = pass
        .open
        .iter()
        .filter(|r| (r[0].1 - 4.5).abs() < 1e-9)
        .collect();
    assert_eq!(at.len(), 2);
    for run in &at {
        let (a, b) = (run[0].0.min(run[1].0), run[0].0.max(run[1].0));
        assert!(b <= 2.5 + 1e-9 || a >= 7.5 - 1e-9, "{run:?}");
    }
    // a line above the hole is whole
    assert_eq!(
        pass.open
            .iter()
            .filter(|r| (r[0].1 - 8.5).abs() < 1e-9)
            .count(),
        1
    );
}

#[test]
fn with_the_copper_given_the_lines_stay_where_the_tool_lies_wholly_on_it() {
    let left_half = Scene {
        shapes: vec![Shape::Region {
            pts: square(0.0, 0.0, 5.0, 10.0),
        }],
        notes: Vec::new(),
    };
    let pass = rubout_contours(
        &[square(0.0, 0.0, 10.0, 10.0)],
        Some(&left_half),
        &fill(1.0, 0.0, Axis::Horizontal),
    )
    .unwrap();
    assert!(!pass.open.is_empty());
    for run in &pass.open {
        for p in run {
            assert!(p.0 <= 5.0 + 1e-9, "a point off the copper: {p:?}");
        }
    }
}

#[test]
fn what_cannot_be_filled_is_refused_with_the_reason() {
    let ok = [square(0.0, 0.0, 10.0, 10.0)];
    let margin = |m: f64| Fill {
        margin_mm: m,
        ..fill(1.0, 0.0, Axis::Horizontal)
    };
    for (f, word) in [
        (fill(0.05, 0.0, Axis::Horizontal), "from 0.1 to 10"),
        (fill(11.0, 0.0, Axis::Horizontal), "from 0.1 to 10"),
        (fill(f64::NAN, 0.0, Axis::Horizontal), "from 0.1 to 10"),
        (fill(1.0, 0.95, Axis::Horizontal), "overlap"),
        (fill(1.0, -0.1, Axis::Horizontal), "overlap"),
        (margin(-1.0), "margin"),
        (margin(11.0), "margin"),
    ] {
        let why = rubout_contours(&ok, None, &f).unwrap_err();
        assert!(why.contains(word), "{why}");
    }
    let plain = fill(1.0, 0.0, Axis::Horizontal);
    // nothing closed to fill, a coordinate that is not a number, a contour thinner than the tool
    let why = rubout_contours(&[vec![(0.0, 0.0), (1.0, 1.0)]], None, &plain).unwrap_err();
    assert!(why.contains("no closed contour"), "{why}");
    let nan = vec![(0.0, 0.0), (f64::NAN, 1.0), (1.0, 0.0)];
    let why = rubout_contours(&[nan], None, &plain).unwrap_err();
    assert!(why.contains("not a number"), "{why}");
    let why = rubout_contours(&[square(0.0, 0.0, 10.0, 0.5)], None, &plain).unwrap_err();
    assert!(why.contains("at least the tool"), "{why}");
    // too many lines
    let tall = fill(0.1, 0.0, Axis::Horizontal);
    let why = rubout_contours(&[square(0.0, 0.0, 1.0, 5000.0)], None, &tall).unwrap_err();
    assert!(why.contains("lines: at most"), "{why}");
}
