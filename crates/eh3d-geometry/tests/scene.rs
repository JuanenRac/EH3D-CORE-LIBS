use eh3d_geometry::scene::{
    arc_points, from_excellon, from_gerber, from_gerber_with, swept_rectangle, Bounds, Scene,
    SceneOptions, Shape,
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

// ---- macro apertures and the tolerance of arcs ----

const ROUND_RECT: &str = "%FSLAX24Y24*%\n%MOMM*%\n%AMRoundRect*\n0 Rectangle with rounded corners*\n0 $1 Rounding radius*\n4,1,4,$2,$3,$4,$5,$6,$7,$8,$9,$2,$3,0*\n1,1,$1+$1,$2,$3*\n1,1,$1+$1,$4,$5*\n1,1,$1+$1,$6,$7*\n1,1,$1+$1,$8,$9*\n20,1,$1+$1,$2,$3,$4,$5,0*\n20,1,$1+$1,$4,$5,$6,$7,0*\n20,1,$1+$1,$6,$7,$8,$9,0*\n20,1,$1+$1,$8,$9,$2,$3,0*%\n%ADD12RoundRect,0.25X0.5X0.25X-0.5X0.25X-0.5X-0.25X0.5X-0.25*%\nD12*\nX100000Y50000D03*\nM02*\n";

#[test]
fn a_rounded_rectangle_macro_is_drawn_where_it_is_flashed() {
    let s = from_gerber(&eh3d_parser_gerber::parse(ROUND_RECT));
    assert!(!s.shapes.is_empty());
    assert!(
        s.notes.iter().all(|n| !n.contains("macro")),
        "{:?}",
        s.notes
    );
    let b = s.bounds();
    // the pad is 1.5 x 1.0 mm around (10, 5)
    assert!(
        (b.min_x - 9.25).abs() < 1e-9 && (b.max_x - 10.75).abs() < 1e-9,
        "{b:?}"
    );
    assert!(
        (b.min_y - 4.5).abs() < 1e-9 && (b.max_y - 5.5).abs() < 1e-9,
        "{b:?}"
    );
}

#[test]
fn a_macro_with_a_variable_a_rotation_and_a_polygon_is_drawn() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%AMTWICE*\n$2=$1x2*\n1,1,$2,0,0*\n21,1,2,1,0,0,90*\n5,1,6,5,0,1,0*%\n%ADD10TWICE,0.5*%\nD10*\nX0Y0D03*\nM02*\n";
    let s = from_gerber(&eh3d_parser_gerber::parse(text));
    assert!(s.notes.is_empty(), "{:?}", s.notes);
    assert!(matches!(s.shapes[0], Shape::Disc { r, .. } if (r - 0.5).abs() < 1e-9));
    // the 2 x 1 bar centred on the origin turned by 90 degrees is 1 wide and 2 high
    let Shape::Region { pts } = &s.shapes[1] else {
        panic!("{:?}", s.shapes[1])
    };
    let mut b = Bounds::empty();
    pts.iter().for_each(|(x, y)| b.include(*x, *y));
    assert!(
        (b.width() - 1.0).abs() < 1e-9 && (b.height() - 2.0).abs() < 1e-9,
        "{b:?}"
    );
    assert!(matches!(&s.shapes[2], Shape::Region { pts } if pts.len() == 6));
}

#[test]
fn a_macro_primitive_that_cannot_be_drawn_is_still_said() {
    for body in [
        "7,0,0,1,0.8,0.1,0",
        "1,0,1,0,0",
        "6,0,0,1,0.1,0.05,2,0.1,1.2,0",
    ] {
        let text = format!(
            "%FSLAX24Y24*%\n%MOMM*%\n%AMODD*\n{body}*%\n%ADD10ODD,1*%\nD10*\nX0Y0D03*\nM02*\n"
        );
        let s = from_gerber(&eh3d_parser_gerber::parse(&text));
        assert!(
            s.notes.iter().any(|n| n.contains("macro aperture")),
            "{body}: {:?}",
            s.notes
        );
    }
}

fn full_circle(radius_mm: f64) -> String {
    let r = (radius_mm * 10_000.0) as i64;
    format!("%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nG75*\nX{r}Y0D02*\nG03*\nX{r}Y0I-{r}J0D01*\nM02*\n")
}

#[test]
fn a_big_arc_stays_within_the_tolerance_of_the_true_circle() {
    let s = from_gerber(&eh3d_parser_gerber::parse(&full_circle(25.0)));
    assert!(s.shapes.len() > 32, "{} pieces", s.shapes.len());
    for shape in &s.shapes {
        let Shape::Stroke { x0, y0, x1, y1, .. } = shape else {
            panic!("{shape:?}")
        };
        assert!((x0.hypot(*y0) - 25.0).abs() < 1e-6);
        // the middle of each chord is the farthest the polygon gets from the arc
        let gap = 25.0 - ((x0 + x1) / 2.0).hypot((y0 + y1) / 2.0);
        assert!(gap <= 0.005 + 1e-9, "the chord is {gap} mm inside the arc");
    }
}

#[test]
fn the_tolerance_of_arcs_is_an_option_and_a_small_arc_keeps_its_32_pieces() {
    let tight = SceneOptions {
        arc_tolerance_mm: 0.001,
    };
    let file = eh3d_parser_gerber::parse(&full_circle(25.0));
    assert!(from_gerber_with(&file, &tight).shapes.len() > from_gerber(&file).shapes.len());
    let small = from_gerber(&eh3d_parser_gerber::parse(&full_circle(0.5)));
    assert_eq!(small.shapes.len(), 32);
}
