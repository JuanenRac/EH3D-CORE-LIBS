use eh3d_geometry::scene::{
    arc_points, from_excellon, from_gerber, swept_rectangle, Bounds, Scene, Shape,
};
use eh3d_parser_gerber::QuadrantMode;

const COPPER: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\n%ADD11R,2.0X1.0*%\nD10*\nX00000Y00000D02*\nX100000Y00000D01*\nD11*\nX50000Y50000D03*\nM02*\n";
const DRILL: &str = "M48\nMETRIC\nT1C0.8\n%\nT1\nX009.000Y009.000\nM30\n";

#[test]
fn an_empty_bounds_has_no_size_and_yields_to_the_other() {
    let e = Bounds::empty();
    assert!(e.is_empty());
    assert_eq!((e.width(), e.height()), (0.0, 0.0));
    let mut b = Bounds::empty();
    b.include(1.0, 2.0);
    b.include(5.0, 4.0);
    assert_eq!((b.width(), b.height(), b.center()), (4.0, 2.0, (3.0, 3.0)));
    assert_eq!(e.union(&b), b);
    assert_eq!(b.union(&e), b);
    let mut c = Bounds::empty();
    c.include(-1.0, 0.0);
    assert_eq!(b.union(&c).min_x, -1.0);
}

#[test]
fn the_bounds_of_a_scene_cover_every_shape_with_its_size() {
    let scene = Scene {
        shapes: vec![
            Shape::Disc {
                x: 0.0,
                y: 0.0,
                r: 1.0,
            },
            Shape::Rect {
                x: 10.0,
                y: 0.0,
                w: 2.0,
                h: 4.0,
            },
            Shape::Stroke {
                x0: 0.0,
                y0: 5.0,
                x1: 0.0,
                y1: 8.0,
                w: 2.0,
            },
            Shape::Region {
                pts: vec![(-3.0, -3.0), (0.0, -2.0), (0.0, 0.0)],
            },
        ],
        notes: Vec::new(),
    };
    let b = scene.bounds();
    assert_eq!((b.min_x, b.max_x), (-3.0, 11.0));
    assert_eq!((b.min_y, b.max_y), (-3.0, 9.0));
    assert!(Scene::default().bounds().is_empty());
}

#[test]
fn a_gerber_layer_and_a_drill_file_become_shapes_in_millimetres() {
    let s = from_gerber(&eh3d_parser_gerber::parse(COPPER));
    assert_eq!(s.shapes.len(), 2);
    assert!(matches!(s.shapes[0], Shape::Stroke { x1, w, .. } if x1 == 10.0 && w == 0.5));
    assert!(matches!(s.shapes[1], Shape::Rect { x, w, h, .. } if x == 5.0 && w == 2.0 && h == 1.0));
    let d = from_excellon(&eh3d_parser_excellon::parse(DRILL));
    assert!(
        matches!(d.shapes[0], Shape::Disc { x, y, r } if x == 9.0 && y == 9.0 && (r - 0.4).abs() < 1e-9)
    );
}

#[test]
fn what_is_not_drawn_is_said_once() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\n%LPC*%\nX0Y0D02*\nX10000Y0D01*\n%LPC*%\nX0Y10000D01*\nM02*\n";
    let s = from_gerber(&eh3d_parser_gerber::parse(text));
    let n = s
        .notes
        .iter()
        .filter(|n| n.contains("clear polarity"))
        .count();
    assert_eq!(n, 1, "{:?}", s.notes);
}

#[test]
fn a_swept_rectangle_is_a_convex_outline_around_both_ends() {
    let pts = swept_rectangle((0.0, 0.0), (4.0, 0.0), 2.0, 2.0);
    let mut b = Bounds::empty();
    pts.iter().for_each(|(x, y)| b.include(*x, *y));
    assert_eq!((b.min_x, b.max_x, b.min_y, b.max_y), (-1.0, 5.0, -1.0, 1.0));
    assert_eq!(pts.len(), 4);
}

#[test]
fn an_arc_ends_where_it_should() {
    let pts = arc_points(
        (1.0, 0.0),
        (-1.0, 0.0),
        (-1.0, 0.0),
        false,
        QuadrantMode::Multi,
    );
    let (first, last) = (pts[0], *pts.last().unwrap());
    assert!((first.0 - 1.0).abs() < 1e-9 && first.1.abs() < 1e-9);
    assert!((last.0 + 1.0).abs() < 1e-9 && last.1.abs() < 1e-9);
    assert!(pts
        .iter()
        .all(|(x, y)| ((x * x + y * y).sqrt() - 1.0).abs() < 1e-9));
    assert!(
        pts.iter().any(|(_, y)| *y > 0.9),
        "counter-clockwise goes over the top"
    );
}

// ---- more edges of the shapes ----

#[test]
fn a_rectangle_dragged_nowhere_is_the_rectangle_and_dragged_along_an_axis_is_a_longer_one() {
    let still = swept_rectangle((3.0, 4.0), (3.0, 4.0), 2.0, 1.0);
    assert_eq!(still.len(), 4);
    assert!((area(&still) - 2.0).abs() < 1e-9);
    let along = swept_rectangle((0.0, 0.0), (10.0, 0.0), 2.0, 1.0);
    assert_eq!(
        along.len(),
        4,
        "the corners in the middle of the long sides are not corners"
    );
    assert!((area(&along) - 12.0).abs() < 1e-9);
    let slanted = swept_rectangle((0.0, 0.0), (10.0, 10.0), 2.0, 1.0);
    assert_eq!(slanted.len(), 6);
    // dragging a rectangle covers its area plus the area it sweeps: |dx| * h + |dy| * w
    assert!(
        (area(&slanted) - (2.0 + 10.0 * 1.0 + 10.0 * 2.0)).abs() < 1e-9,
        "{}",
        area(&slanted)
    );
}

#[test]
fn the_dragged_rectangle_runs_counterclockwise_whichever_way_it_is_dragged() {
    for (a, b) in [
        ((0.0, 0.0), (5.0, 3.0)),
        ((5.0, 3.0), (0.0, 0.0)),
        ((0.0, 0.0), (-5.0, 3.0)),
        ((0.0, 0.0), (0.0, -7.0)),
    ] {
        let poly = swept_rectangle(a, b, 1.0, 2.0);
        assert!(area(&poly) > 0.0, "{a:?} to {b:?}");
        let mirror = swept_rectangle(b, a, 1.0, 2.0);
        assert!(
            (area(&poly) - area(&mirror)).abs() < 1e-9,
            "the direction of the drag does not change the shape"
        );
    }
}

#[test]
fn a_rectangle_with_no_size_does_not_panic() {
    for (w, h) in [(0.0, 0.0), (0.0, 2.0), (2.0, 0.0)] {
        let poly = swept_rectangle((0.0, 0.0), (4.0, 1.0), w, h);
        assert!(poly.len() <= 6);
        assert!(poly.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
    }
}

fn area(p: &[(f64, f64)]) -> f64 {
    (0..p.len())
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % p.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0
}

#[test]
fn a_union_with_an_empty_box_is_the_other_and_a_box_with_a_point_has_no_size() {
    let mut one = Bounds::empty();
    one.include(2.0, 3.0);
    assert!(!one.is_empty());
    assert_eq!((one.width(), one.height()), (0.0, 0.0));
    assert_eq!(Bounds::empty().union(&Bounds::empty()), Bounds::empty());
    assert!(Bounds::empty().union(&Bounds::empty()).is_empty());
    let mut a = Bounds::empty();
    a.include(0.0, 0.0);
    a.include(2.0, 2.0);
    let mut b = Bounds::empty();
    b.include(5.0, -1.0);
    b.include(6.0, 1.0);
    let u = a.union(&b);
    assert_eq!((u.min_x, u.min_y, u.max_x, u.max_y), (0.0, -1.0, 6.0, 2.0));
    assert_eq!(u, b.union(&a));
}

#[test]
fn the_box_of_a_stroke_is_its_two_ends_widened_by_half_the_pen() {
    let s = Scene {
        shapes: vec![Shape::Stroke {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 4.0,
            w: 1.0,
        }],
        notes: vec![],
    };
    let b = s.bounds();
    assert_eq!(
        (b.min_x, b.min_y, b.max_x, b.max_y),
        (-0.5, -0.5, 10.5, 4.5)
    );
    assert!(Scene::default().bounds().is_empty());
}

#[test]
fn the_same_gerber_file_gives_the_same_scene_every_time() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\n%ADD11R,2.0X1.0*%\nD10*\nX0Y0D02*\nX100000Y50000D01*\nD11*\nX50000Y50000D03*\nG36*\nX0Y0D02*\nX10000Y0D01*\nX10000Y10000D01*\nG37*\nM02*\n";
    let a = from_gerber(&eh3d_parser_gerber::parse(text));
    let b = from_gerber(&eh3d_parser_gerber::parse(text));
    assert_eq!(a, b);
    assert!(a.shapes.iter().any(|s| matches!(s, Shape::Region { .. })));
}

#[test]
fn drill_files_in_inches_give_discs_in_millimetres_and_an_empty_file_gives_an_empty_scene() {
    let scene = from_excellon(&eh3d_parser_excellon::parse(
        "M48\nINCH\nT1C0.04\n%\nT1\nX1.0Y2.0\nM30\n",
    ));
    match &scene.shapes[0] {
        Shape::Disc { x, y, r } => assert!(
            (x - 25.4).abs() < 1e-9 && (y - 50.8).abs() < 1e-9 && (r - 0.508).abs() < 1e-9,
            "{x} {y} {r}"
        ),
        other => panic!("{other:?}"),
    }
    assert!(from_excellon(&eh3d_parser_excellon::parse(""))
        .shapes
        .is_empty());
    assert!(from_gerber(&eh3d_parser_gerber::parse(""))
        .shapes
        .is_empty());
}
