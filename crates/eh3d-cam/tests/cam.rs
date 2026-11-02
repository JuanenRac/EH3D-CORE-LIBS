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
