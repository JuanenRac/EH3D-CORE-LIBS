use eh3d_cam::tolerance::{close_pieces_scoped, is_pad, ToleranceScope};
use eh3d_geometry::scene::{Scene, Shape};

const RES: f64 = 0.02;

/// Two discs 0.2 mm apart (edge to edge): pads.
fn two_pads() -> Scene {
    Scene {
        shapes: vec![
            Shape::Disc {
                x: 0.0,
                y: 0.0,
                r: 1.0,
            },
            Shape::Disc {
                x: 2.2,
                y: 0.0,
                r: 1.0,
            },
        ],
        notes: Vec::new(),
    }
}

/// Two tracks 0.2 mm apart (edge to edge): no pad in the scene.
fn two_tracks() -> Scene {
    Scene {
        shapes: vec![
            Shape::Stroke {
                x0: 0.0,
                y0: 0.0,
                x1: 4.0,
                y1: 0.0,
                w: 0.3,
            },
            Shape::Stroke {
                x0: 0.0,
                y0: 0.5,
                x1: 4.0,
                y1: 0.5,
                w: 0.3,
            },
        ],
        notes: Vec::new(),
    }
}

#[test]
fn the_scope_is_read_from_its_words_and_named_back() {
    for (word, scope) in [
        ("all", ToleranceScope::All),
        (" Everything ", ToleranceScope::All),
        ("ALL_OBJECTS", ToleranceScope::All),
        ("pads", ToleranceScope::Pads),
        ("pad", ToleranceScope::Pads),
        ("only_pads", ToleranceScope::Pads),
    ] {
        assert_eq!(ToleranceScope::parse(word), Ok(scope), "{word}");
    }
    assert_eq!(ToleranceScope::default(), ToleranceScope::All);
    assert_eq!(
        ToleranceScope::parse(ToleranceScope::Pads.name()),
        Ok(ToleranceScope::Pads)
    );
    assert_eq!(
        ToleranceScope::parse(ToleranceScope::All.name()),
        Ok(ToleranceScope::All)
    );
    let why = ToleranceScope::parse("tracks").unwrap_err();
    assert!(
        why.contains("tracks") && why.contains("all or pads"),
        "{why}"
    );
}

#[test]
fn a_pad_is_a_flash_and_a_track_or_a_region_is_not() {
    assert!(is_pad(&Shape::Disc {
        x: 0.0,
        y: 0.0,
        r: 1.0
    }));
    assert!(is_pad(&Shape::Rect {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0
    }));
    assert!(is_pad(&Shape::Obround {
        x: 0.0,
        y: 0.0,
        w: 2.0,
        h: 1.0
    }));
    assert!(!is_pad(&Shape::Stroke {
        x0: 0.0,
        y0: 0.0,
        x1: 1.0,
        y1: 0.0,
        w: 0.2
    }));
    assert!(!is_pad(&Shape::Region {
        pts: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]
    }));
}

#[test]
fn without_tolerance_a_gap_narrower_than_the_tool_is_a_conflict_for_every_scope() {
    for scope in [ToleranceScope::All, ToleranceScope::Pads] {
        for scene in [two_pads(), two_tracks()] {
            let found = close_pieces_scoped(&scene, 0.3, 0.0, scope, RES).unwrap();
            assert!(!found.is_empty(), "{scope:?}");
            assert!(found.iter().all(|c| c.pieces >= 2));
        }
    }
}

#[test]
fn a_tolerance_that_lets_the_tool_through_clears_the_gap_for_all_objects() {
    for scene in [two_pads(), two_tracks()] {
        let found = close_pieces_scoped(&scene, 0.3, 0.15, ToleranceScope::All, RES).unwrap();
        assert!(found.is_empty(), "{found:?}");
    }
}

#[test]
fn with_the_pads_scope_the_tolerance_clears_a_gap_beside_a_pad_but_not_between_tracks() {
    let pads = close_pieces_scoped(&two_pads(), 0.3, 0.15, ToleranceScope::Pads, RES).unwrap();
    assert!(pads.is_empty(), "{pads:?}");
    let tracks = close_pieces_scoped(&two_tracks(), 0.3, 0.15, ToleranceScope::Pads, RES).unwrap();
    assert!(!tracks.is_empty(), "a track must not be scraped");
}

#[test]
fn a_gap_wider_than_the_tool_is_never_a_conflict_and_a_bad_resolution_is_refused() {
    for scope in [ToleranceScope::All, ToleranceScope::Pads] {
        let found = close_pieces_scoped(&two_pads(), 0.1, 0.0, scope, RES).unwrap();
        assert!(found.is_empty(), "{scope:?} {found:?}");
    }
    assert!(close_pieces_scoped(&two_pads(), 0.3, 0.0, ToleranceScope::All, 0.0).is_err());
}
