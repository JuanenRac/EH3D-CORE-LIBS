use eh3d_cam::problems::{
    corner_residue_mm2, problem_zones, thin_for_tolerance, thin_for_tolerance_in, Problem,
    ProblemKind, ProblemRules,
};
use eh3d_geometry::scene::{Scene, Shape};

fn rules() -> ProblemRules {
    ProblemRules {
        tool_mm: 0.2,
        corner_angle_deg: 0.0,
        track_gap_factor: 0.0,
    }
}

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

#[test]
fn rules_outside_their_range_are_refused_with_the_name_of_the_rule() {
    assert!(rules().validate().is_ok());
    for (rule, name) in [
        (
            ProblemRules {
                tool_mm: 0.0,
                ..rules()
            },
            "fine_tool_mm",
        ),
        (
            ProblemRules {
                tool_mm: 11.0,
                ..rules()
            },
            "fine_tool_mm",
        ),
        (
            ProblemRules {
                tool_mm: f64::NAN,
                ..rules()
            },
            "fine_tool_mm",
        ),
        (
            ProblemRules {
                corner_angle_deg: 5.0,
                ..rules()
            },
            "problem_corner_angle_deg",
        ),
        (
            ProblemRules {
                corner_angle_deg: 90.0,
                ..rules()
            },
            "problem_corner_angle_deg",
        ),
        (
            ProblemRules {
                track_gap_factor: -1.0,
                ..rules()
            },
            "problem_track_gap_factor",
        ),
        (
            ProblemRules {
                track_gap_factor: 11.0,
                ..rules()
            },
            "problem_track_gap_factor",
        ),
    ] {
        let why = rule.validate().unwrap_err();
        assert!(why.contains(name), "{why}");
        assert!(problem_zones(&scene(vec![]), &rule, 0.05).is_err());
    }
    // a corner angle of 0 means off, and so is a gap factor of 0
    assert!(ProblemRules {
        corner_angle_deg: 0.0,
        track_gap_factor: 0.0,
        ..rules()
    }
    .validate()
    .is_ok());
}

#[test]
fn a_cell_out_of_range_or_too_coarse_for_the_corners_is_refused() {
    let s = scene(vec![Shape::Disc {
        x: 0.0,
        y: 0.0,
        r: 1.0,
    }]);
    for res in [0.0, 0.001, 2.0, f64::NAN] {
        assert!(problem_zones(&s, &rules(), res).is_err(), "{res}");
    }
    let looking = ProblemRules {
        corner_angle_deg: 45.0,
        ..rules()
    };
    let why = problem_zones(&s, &looking, 0.15).unwrap_err();
    assert!(why.contains("too coarse"), "{why}");
    // with the corners off a coarser cell is not a problem
    assert!(problem_zones(&s, &rules(), 0.15).is_ok());
}

#[test]
fn with_both_searches_off_nothing_is_reported() {
    let s = scene(vec![Shape::Disc {
        x: 0.0,
        y: 0.0,
        r: 1.0,
    }]);
    assert_eq!(problem_zones(&s, &rules(), 0.05).unwrap(), vec![]);
}

#[test]
fn the_copper_left_in_a_corner_grows_as_the_corner_gets_sharper() {
    // a square corner of 90 degrees and a tool of radius 1 leaves 1 - pi/4 of area
    let square = corner_residue_mm2(1.0, 90.0);
    assert!(
        (square - (1.0 - std::f64::consts::FRAC_PI_4)).abs() < 1e-12,
        "{square}"
    );
    let mut last = 0.0;
    for angle in [80.0, 60.0, 45.0, 30.0, 15.0] {
        let now = corner_residue_mm2(1.0, angle);
        assert!(now > last, "{angle}: {now} after {last}");
        last = now;
    }
    // the residue goes with the square of the radius
    assert!((corner_residue_mm2(2.0, 45.0) - 4.0 * corner_residue_mm2(1.0, 45.0)).abs() < 1e-9);
}

#[test]
fn a_track_between_two_pads_that_are_near_it_is_a_problem_and_a_lone_track_is_not() {
    let rule = ProblemRules {
        track_gap_factor: 2.0,
        ..rules()
    };
    let track = Shape::Stroke {
        x0: -2.0,
        y0: 0.0,
        x1: 2.0,
        y1: 0.0,
        w: 0.2,
    };
    let lone = scene(vec![track.clone()]);
    assert!(problem_zones(&lone, &rule, 0.05).unwrap().is_empty());
    // 0.15 mm between the edge of each pad and the edge of the track: less than twice the tool
    let between = scene(vec![
        track,
        Shape::Disc {
            x: 0.0,
            y: 0.75,
            r: 0.5,
        },
        Shape::Disc {
            x: 0.0,
            y: -0.75,
            r: 0.5,
        },
    ]);
    let found = problem_zones(&between, &rule, 0.05).unwrap();
    assert!(
        found
            .iter()
            .any(|p| p.kind == ProblemKind::TrackBetweenPads),
        "{found:?}"
    );
    let p: &Problem = found
        .iter()
        .find(|p| p.kind == ProblemKind::TrackBetweenPads)
        .unwrap();
    assert!(p.describe().contains("track between two pads"));
}

#[test]
fn a_problem_says_what_it_is_in_words() {
    let corner = Problem {
        kind: ProblemKind::SharpCorner,
        x: 1.0,
        y: 2.0,
        value: 45.0,
    };
    assert!(corner.describe().contains("sharper than 45 degrees"));
    assert!(corner.describe().contains("(1.000, 2.000)"));
}

#[test]
fn copper_thinner_than_twice_the_tolerance_is_told_from_the_thinnest() {
    let s = scene(vec![
        Shape::Stroke {
            x0: 0.0,
            y0: 0.0,
            x1: 3.0,
            y1: 0.0,
            w: 0.15,
        },
        Shape::Stroke {
            x0: 0.0,
            y0: 1.0,
            x1: 3.0,
            y1: 1.0,
            w: 0.3,
        },
        Shape::Rect {
            x: 5.0,
            y: 5.0,
            w: 0.5,
            h: 0.1,
        },
        Shape::Disc {
            x: 8.0,
            y: 8.0,
            r: 0.01,
        },
    ]);
    // tolerance 0.1: twice is 0.2, so the track of 0.15 and the pad of 0.1 are thin, the track of 0.3 is not, and a disc is never listed
    let thin = thin_for_tolerance(&s, 0.1, 0.0).unwrap();
    let sizes: Vec<f64> = thin.iter().map(|t| t.size_mm).collect();
    assert_eq!(sizes, vec![0.1, 0.15]);
    // the margin raises the limit
    let wider = thin_for_tolerance(&s, 0.1, 0.15).unwrap();
    assert_eq!(wider.len(), 3);
    // with the pads only the tracks are left out
    let pads = thin_for_tolerance_in(&s, 0.1, 0.0, true).unwrap();
    assert_eq!(
        pads.iter().map(|t| t.size_mm).collect::<Vec<_>>(),
        vec![0.1]
    );
    // no tolerance, nothing to tell
    assert!(thin_for_tolerance(&s, 0.0, 0.0).unwrap().is_empty());
}

#[test]
fn a_tolerance_or_a_margin_that_is_not_a_number_or_is_negative_is_refused() {
    let s = scene(vec![]);
    for (t, m) in [
        (-0.1, 0.0),
        (f64::NAN, 0.0),
        (0.1, -0.1),
        (0.1, f64::INFINITY),
    ] {
        assert!(thin_for_tolerance(&s, t, m).is_err(), "{t} {m}");
    }
}
