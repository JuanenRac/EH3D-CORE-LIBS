use eh3d_cam::bitmap::{capsule_distance, from_scene, inside_polygon};
use eh3d_cam::hole::Hole;
use eh3d_cam::order::{is_clockwise, signed_area};
use eh3d_geometry::scene::{Scene, Shape};

#[test]
fn a_disc_is_drawn_on_the_grid_with_about_its_area() {
    let scene = Scene {
        shapes: vec![Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 2.0,
        }],
        notes: Vec::new(),
    };
    let bitmap = from_scene(&scene, 0.5, 0.05).unwrap();
    let area = bitmap.count() as f64 * 0.05 * 0.05;
    let exact = std::f64::consts::PI * 4.0;
    assert!(
        (area - exact).abs() / exact < 0.02,
        "{area} against {exact}"
    );
    assert_eq!(bitmap.at(0.0, 0.0), Some(true));
    assert_eq!(bitmap.at(2.4, 0.0), Some(false));
}

#[test]
fn the_helpers_of_the_grid_measure_distance_and_containment() {
    assert!((capsule_distance(0.0, 3.0, -1.0, 0.0, 1.0, 0.0) - 3.0).abs() < 1e-12);
    assert!((capsule_distance(3.0, 0.0, -1.0, 0.0, 1.0, 0.0) - 2.0).abs() < 1e-12);
    let square = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
    assert!(inside_polygon(1.0, 1.0, &square));
    assert!(!inside_polygon(3.0, 1.0, &square));
}

#[test]
fn the_direction_of_a_path_comes_from_its_signed_area() {
    let ccw = [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
    assert!((signed_area(&ccw) - 4.0).abs() < 1e-12);
    assert!(!is_clockwise(&ccw));
    let cw: Vec<_> = ccw.iter().rev().cloned().collect();
    assert!(is_clockwise(&cw));
}

#[test]
fn a_hole_is_plain_data() {
    let h = Hole {
        tool: 1,
        x_mm: 2.5,
        y_mm: 3.5,
        diameter_mm: 0.8,
    };
    assert_eq!(h, h);
}

fn stroke(x0: f64, y0: f64, x1: f64, y1: f64, w: f64) -> Shape {
    Shape::Stroke { x0, y0, x1, y1, w }
}

#[test]
fn a_track_draws_the_same_cells_as_testing_every_cell_of_its_box() {
    let shapes = vec![
        stroke(1.0, 1.0, 19.0, 14.0, 0.4),
        stroke(2.0, 15.0, 18.0, 2.0, 0.25),
        stroke(5.0, 8.0, 5.0, 8.0, 1.0),
        stroke(3.0, 3.0, 17.0, 3.0, 0.3),
        stroke(9.0, 1.0, 9.5, 15.0, 0.31),
        stroke(1.0, 14.9, 19.1, 14.9, 0.05),
    ];
    let scene = Scene {
        shapes: shapes.clone(),
        notes: Vec::new(),
    };
    let bm = from_scene(&scene, 0.5, 0.05).unwrap();
    for iy in 0..bm.h {
        for ix in 0..bm.w {
            let (x, y) = bm.centre(ix as f64, iy as f64);
            let want = shapes.iter().any(|s| match s {
                Shape::Stroke { x0, y0, x1, y1, w } => {
                    capsule_distance(x, y, *x0, *y0, *x1, *y1) <= w / 2.0
                }
                _ => false,
            });
            assert_eq!(bm.on[iy * bm.w + ix], want, "cell {ix},{iy}");
        }
    }
}

#[test]
fn many_long_diagonal_tracks_cost_their_length_and_not_the_area_of_their_boxes() {
    // 100 tracks across a board of 100 x 80 mm at 0.02 mm: testing every cell of every box is 2 billion tests
    let shapes = (0..100)
        .map(|i| stroke(0.0, 0.8 * i as f64, 100.0, 80.0 - 0.8 * i as f64, 0.2))
        .collect();
    let scene = Scene {
        shapes,
        notes: Vec::new(),
    };
    let started = std::time::Instant::now();
    let bm = from_scene(&scene, 0.5, 0.02).unwrap();
    assert!(bm.count() > 100_000);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_size_out_of_range_is_refused_and_does_not_wrap_around() {
    let mut b = eh3d_geometry::scene::Bounds::empty();
    b.include(0.0, 0.0);
    b.include(1e17, 1.0);
    assert!(eh3d_cam::bitmap::Bitmap::new(&b, 0.0, 0.002).is_err());
    b.include(1e300, 1.0);
    assert!(eh3d_cam::bitmap::Bitmap::new(&b, 0.0, 0.002).is_err());
}

#[test]
fn the_pieces_that_come_close_are_found_among_many_without_going_through_the_grid_for_each() {
    // 40 x 40 separate pads 2 mm apart and one pair 0.1 mm apart: a gap of 0.2 mm is too close only there
    let mut shapes: Vec<Shape> = Vec::new();
    for i in 0..40 {
        for j in 0..40 {
            shapes.push(Shape::Disc {
                x: 2.0 * i as f64,
                y: 2.0 * j as f64,
                r: 0.3,
            });
        }
    }
    shapes.push(Shape::Disc {
        x: 0.7,
        y: 0.0,
        r: 0.3,
    });
    let scene = Scene {
        shapes,
        notes: Vec::new(),
    };
    let started = std::time::Instant::now();
    let close = eh3d_cam::isolate::close_pieces(&scene, 0.2, 0.05).unwrap();
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(close.len(), 1, "{close:?}");
    assert_eq!(close[0].pieces, 2);
    assert!(
        (close[0].x - 0.35).abs() < 0.1 && close[0].y.abs() < 0.1,
        "{close:?}"
    );
}
