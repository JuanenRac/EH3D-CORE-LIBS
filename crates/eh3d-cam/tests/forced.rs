use eh3d_cam::forced::{forced_passes, Forced};
use eh3d_cam::isolate::{Plan, Tool};
use eh3d_geometry::scene::{Scene, Shape};

fn plan() -> Plan {
    Plan {
        tools: vec![Tool {
            diameter_mm: 0.4,
            passes: 1,
            stepover_mm: 0.1,
        }],
        rubout: None,
        res_mm: 0.02,
        trim: false,
    }
}

/// Two discs of radius 1 whose edges are `gap` mm apart.
fn two_discs(gap: f64) -> Scene {
    Scene {
        shapes: vec![
            Shape::Disc {
                x: 0.0,
                y: 0.0,
                r: 1.0,
            },
            Shape::Disc {
                x: 2.0 + gap,
                y: 0.0,
                r: 1.0,
            },
        ],
        notes: Vec::new(),
    }
}

#[test]
fn the_minimum_gap_must_be_inside_its_range_and_between_two_cells_and_the_finest_tool() {
    let p = plan();
    assert!(Forced { min_gap_mm: 0.1 }.validate(&p).is_ok());
    for (gap, word) in [
        (0.01, "forced_min_gap_mm"),
        (11.0, "forced_min_gap_mm"),
        (f64::NAN, "forced_min_gap_mm"),
        (0.03, "two cells"),
        (0.4, "finest tool"),
        (0.5, "finest tool"),
    ] {
        let why = Forced { min_gap_mm: gap }.validate(&p).unwrap_err();
        assert!(why.contains(word), "{gap}: {why}");
    }
}

#[test]
fn one_piece_of_copper_has_no_gap_to_force() {
    let one = Scene {
        shapes: vec![Shape::Disc {
            x: 0.0,
            y: 0.0,
            r: 1.0,
        }],
        notes: Vec::new(),
    };
    let found = forced_passes(&one, &plan(), &Forced { min_gap_mm: 0.1 }).unwrap();
    assert!(found.passes.is_empty() && found.forced.is_empty() && found.too_narrow.is_empty());
}

#[test]
fn a_gap_narrower_than_the_tool_but_not_than_the_minimum_gets_a_path_where_the_copper_is_near() {
    let found = forced_passes(&two_discs(0.2), &plan(), &Forced { min_gap_mm: 0.1 }).unwrap();
    assert!(!found.passes.is_empty(), "{found:?}");
    assert!(
        !found.forced.is_empty() && found.too_narrow.is_empty(),
        "{found:?}"
    );
    // the gap widens away from the line between the two centres (the edges are round): every place is narrower than the tool and not narrower than the minimum
    for site in &found.forced {
        assert!(site.gap_mm < 0.4 && site.gap_mm >= 0.09, "{site:?}");
        assert!((0.8..=1.5).contains(&site.x), "{site:?}");
        assert!(site.length_mm > 0.0);
    }
    let narrowest = found
        .forced
        .iter()
        .map(|s| s.gap_mm)
        .fold(f64::INFINITY, f64::min);
    assert!((0.1..=0.3).contains(&narrowest), "{narrowest}");
}

#[test]
fn a_gap_narrower_than_the_minimum_is_listed_and_gets_no_path_there() {
    let found = forced_passes(&two_discs(0.06), &plan(), &Forced { min_gap_mm: 0.1 }).unwrap();
    assert!(!found.too_narrow.is_empty(), "{found:?}");
    assert!(found
        .too_narrow
        .iter()
        .all(|s| s.length_mm == 0.0 && s.gap_mm < 0.1));
    let at_the_middle = found
        .too_narrow
        .iter()
        .any(|s| s.y.abs() < 0.1 && (s.gap_mm - 0.06).abs() < 0.03);
    assert!(at_the_middle, "{:?}", found.too_narrow);
    // the places where the gap is wide enough still get their path
    assert!(
        found.forced.iter().all(|s| s.gap_mm >= 0.09),
        "{:?}",
        found.forced
    );
}

#[test]
fn a_gap_wide_enough_for_the_tool_is_left_alone() {
    let found = forced_passes(&two_discs(0.6), &plan(), &Forced { min_gap_mm: 0.1 }).unwrap();
    assert!(found.passes.is_empty() && found.forced.is_empty() && found.too_narrow.is_empty());
}
