use eh3d_cam::drc::{check, Kind, Rules};
use eh3d_cam::hole::Hole;
use eh3d_geometry::scene::{Scene, Shape};

const RES: f64 = 0.02;

fn rules() -> Rules {
    Rules {
        min_width_mm: 0.2,
        min_clearance_mm: 0.15,
        annular_ring_mm: 0.0,
        tool_mm: 0.2,
        holes_on_copper: false,
    }
}

fn track(y: f64, w: f64) -> Shape {
    Shape::Stroke {
        x0: 0.0,
        y0: y,
        x1: 6.0,
        y1: y,
        w,
    }
}

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

fn kinds(found: &[eh3d_cam::drc::Violation]) -> Vec<Kind> {
    let mut k: Vec<Kind> = found.iter().map(|v| v.kind).collect();
    k.sort();
    k.dedup();
    k
}

#[test]
fn rules_outside_their_range_and_a_cell_too_coarse_are_refused_with_the_name() {
    for (rule, name) in [
        (
            Rules {
                min_width_mm: 0.0,
                ..rules()
            },
            "rule_min_width_mm",
        ),
        (
            Rules {
                min_clearance_mm: 9.0,
                ..rules()
            },
            "rule_min_clearance_mm",
        ),
        (
            Rules {
                annular_ring_mm: -1.0,
                ..rules()
            },
            "rule_annular_ring_mm",
        ),
        (
            Rules {
                tool_mm: f64::NAN,
                ..rules()
            },
            "fine_tool_mm",
        ),
    ] {
        let why = rule.validate().unwrap_err();
        assert!(why.contains(name), "{why}");
        assert!(check(&scene(vec![track(0.0, 0.4)]), &[], &rule, RES).is_err());
    }
    assert!(rules().validate().is_ok());
    let s = scene(vec![track(0.0, 0.4)]);
    assert!(check(&s, &[], &rules(), 0.0).is_err());
    assert!(check(&s, &[], &rules(), 2.0).is_err());
    let why = check(&s, &[], &rules(), 0.1).unwrap_err();
    assert!(why.contains("too coarse"), "{why}");
}

#[test]
fn a_wide_track_alone_breaks_no_rule() {
    let found = check(&scene(vec![track(0.0, 0.4)]), &[], &rules(), RES).unwrap();
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn two_tracks_closer_than_the_rule_are_a_clearance_violation_near_the_gap() {
    let s = scene(vec![track(0.0, 0.4), track(0.5, 0.4)]);
    let found = check(&s, &[], &rules(), RES).unwrap();
    assert!(kinds(&found).contains(&Kind::Clearance), "{found:?}");
    let v = found.iter().find(|v| v.kind == Kind::Clearance).unwrap();
    assert!(
        (0.0..=6.0).contains(&v.x) && (0.0..=0.5).contains(&v.y),
        "{v:?}"
    );
    assert_eq!(v.limit_mm, 0.15);
    assert!(v.describe().contains("clearance"));
}

#[test]
fn a_gap_that_meets_the_rule_but_not_the_tool_is_told_for_the_tool_only() {
    // 0.17 mm of gap: more than the rule (0.1) and less than the tool (0.25)
    let rule = Rules {
        min_clearance_mm: 0.1,
        tool_mm: 0.25,
        ..rules()
    };
    let s = scene(vec![track(0.0, 0.4), track(0.57, 0.4)]);
    let found = check(&s, &[], &rule, RES).unwrap();
    let k = kinds(&found);
    assert!(k.contains(&Kind::ClearanceForTool), "{found:?}");
    assert!(!k.contains(&Kind::Clearance), "{found:?}");
}

#[test]
fn copper_narrower_than_the_width_rule_is_reported() {
    let found = check(&scene(vec![track(0.0, 0.1)]), &[], &rules(), RES).unwrap();
    assert!(kinds(&found).contains(&Kind::Width), "{found:?}");
    assert!(found.iter().any(|v| v.describe().contains("narrower")));
}

#[test]
fn the_ring_round_a_hole_and_a_hole_off_the_copper_are_checked_only_when_asked() {
    let pad = |r: f64| scene(vec![Shape::Disc { x: 5.0, y: 5.0, r }]);
    let hole = |x: f64| Hole {
        tool: 1,
        x_mm: x,
        y_mm: 5.0,
        diameter_mm: 0.8,
    };
    let with_ring = Rules {
        annular_ring_mm: 0.3,
        holes_on_copper: true,
        ..rules()
    };
    // a pad of 1.0 mm radius leaves 0.6 mm of ring round a 0.8 mm hole: fine
    let found = check(&pad(1.0), &[hole(5.0)], &with_ring, RES).unwrap();
    assert!(!kinds(&found).contains(&Kind::AnnularRing), "{found:?}");
    // a pad of 0.6 mm radius leaves 0.2 mm: less than the 0.3 mm of the rule
    let found = check(&pad(0.6), &[hole(5.0)], &with_ring, RES).unwrap();
    assert!(kinds(&found).contains(&Kind::AnnularRing), "{found:?}");
    // a hole far from any copper
    let found = check(&pad(1.0), &[hole(9.0)], &with_ring, RES).unwrap();
    assert!(kinds(&found).contains(&Kind::HoleOffCopper), "{found:?}");
    // the same hole is not reported when the rule is off
    let off = Rules {
        holes_on_copper: false,
        ..with_ring
    };
    let found = check(&pad(1.0), &[hole(9.0)], &off, RES).unwrap();
    assert!(!kinds(&found).contains(&Kind::HoleOffCopper));
    // a hole with no diameter cannot have its ring checked
    let nameless = Hole {
        diameter_mm: 0.0,
        ..hole(5.0)
    };
    assert!(check(&pad(1.0), &[nameless], &with_ring, RES).is_err());
}
