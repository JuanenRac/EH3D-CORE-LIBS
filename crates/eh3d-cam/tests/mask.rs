use eh3d_cam::mask::{openings, write_gerber};
use eh3d_geometry::scene::{Scene, Shape};

fn copper() -> Scene {
    Scene {
        shapes: vec![
            Shape::Disc {
                x: 1.0,
                y: 1.0,
                r: 0.5,
            },
            Shape::Rect {
                x: 4.0,
                y: 1.0,
                w: 1.0,
                h: 2.0,
            },
            Shape::Stroke {
                x0: 0.0,
                y0: 0.0,
                x1: 5.0,
                y1: 0.0,
                w: 0.3,
            },
        ],
        notes: Vec::new(),
    }
}

#[test]
fn without_a_mask_layer_the_pads_are_opened_grown_by_the_margin_and_the_tracks_stay_covered() {
    let open = openings(&copper(), None, 0.05).unwrap();
    assert_eq!(open.shapes.len(), 2, "{:?}", open.shapes);
    match &open.shapes[0] {
        Shape::Disc { r, .. } => assert!((r - 0.55).abs() < 1e-9),
        other => panic!("{other:?}"),
    }
    match &open.shapes[1] {
        Shape::Rect { w, h, .. } => assert!((w - 1.1).abs() < 1e-9 && (h - 2.1).abs() < 1e-9),
        other => panic!("{other:?}"),
    }
}

#[test]
fn with_a_mask_layer_its_shapes_are_grown_and_a_negative_margin_can_remove_a_small_one() {
    let mask = Scene {
        shapes: vec![
            Shape::Disc {
                x: 0.0,
                y: 0.0,
                r: 0.1,
            },
            Shape::Disc {
                x: 3.0,
                y: 0.0,
                r: 1.0,
            },
        ],
        notes: Vec::new(),
    };
    let grown = openings(&copper(), Some(&mask), 0.1).unwrap();
    assert_eq!(grown.shapes.len(), 2);
    let shrunk = openings(&copper(), Some(&mask), -0.2).unwrap();
    assert_eq!(
        shrunk.shapes.len(),
        1,
        "the small opening disappears: {:?}",
        shrunk.shapes
    );
}

#[test]
fn a_margin_out_of_range_is_refused_with_its_name() {
    for margin in [1.5, -1.5, f64::NAN, f64::INFINITY] {
        let why = openings(&copper(), None, margin).unwrap_err();
        assert!(why.contains("mask_margin_mm"), "{why}");
    }
}

#[test]
fn the_openings_are_written_as_a_gerber_layer_with_one_aperture_for_the_same_size() {
    let scene = Scene {
        shapes: vec![
            Shape::Disc {
                x: 1.0,
                y: 1.0,
                r: 0.55,
            },
            Shape::Disc {
                x: 2.0,
                y: 1.0,
                r: 0.55,
            },
            Shape::Rect {
                x: 4.0,
                y: 1.0,
                w: 1.1,
                h: 2.1,
            },
            Shape::Obround {
                x: 6.0,
                y: 1.0,
                w: 1.0,
                h: 2.0,
            },
        ],
        notes: Vec::new(),
    };
    let text = write_gerber(&scene).unwrap();
    assert_eq!(text.matches("D03*").count(), 4);
    assert_eq!(text.matches("C,1.100000").count(), 1, "{text}");
    assert!(
        text.contains("R,1.100000X2.100000") && text.contains("O,1.000000X2.000000"),
        "{text}"
    );
    assert!(text.contains("X1000000Y1000000D03*"), "{text}");
}

#[test]
fn a_shape_that_cannot_be_written_is_refused() {
    let nan = Scene {
        shapes: vec![Shape::Disc {
            x: f64::NAN,
            y: 0.0,
            r: 1.0,
        }],
        notes: Vec::new(),
    };
    assert!(write_gerber(&nan).unwrap_err().contains("not a number"));
    let far = Scene {
        shapes: vec![Shape::Disc {
            x: 1.0e7,
            y: 0.0,
            r: 1.0,
        }],
        notes: Vec::new(),
    };
    assert!(write_gerber(&far).unwrap_err().contains("too far"));
}
