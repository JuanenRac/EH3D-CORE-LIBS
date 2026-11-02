use eh3d_cam::fiducial::{
    apply_to_holes, apply_to_passes, fit, parse_pair, Affine, Limits, Model, Pair,
};
use eh3d_cam::hole::Hole;
use eh3d_cam::isolate::Pass;
use eh3d_cam::optimize::{optimize_passes, optimize_path, Optimize};
use eh3d_cam::order::{
    arrange, directions, signed_area, travel_mm, Cut, Direction, LoopStart, Order, Strategy,
};
use eh3d_cam::transform::{
    alignment_holes, alignment_pattern, lay_out, mirror_holes, mirror_scene, panel_offsets,
    rotate_holes, rotate_scene, Axis, Board, HolePattern, Layout,
};
use eh3d_geometry::scene::{Bounds, Scene, Shape};
use eh3d_raster::units::Unit;

fn close(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
}

fn pair(d: (f64, f64), m: (f64, f64)) -> Pair {
    Pair {
        design: d,
        measured: m,
    }
}

fn square(x: f64, y: f64, s: f64) -> Cut {
    Cut {
        pts: vec![(x, y), (x + s, y), (x + s, y + s), (x, y + s)],
        closed: true,
    }
}

fn bounds(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
    let mut b = Bounds::empty();
    b.include(x0, y0);
    b.include(x1, y1);
    b
}

// ---- fiducials --------------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn two_marks_that_only_moved_give_a_pure_shift() {
    let f = fit(
        &[
            pair((0.0, 0.0), (1.5, -2.0)),
            pair((10.0, 0.0), (11.5, -2.0)),
        ],
        Model::Auto,
        &Limits::default(),
    )
    .unwrap();
    assert!(close(f.affine.apply((3.0, 4.0)), (4.5, 2.0)));
    assert!(
        (f.scale_x - 1.0).abs() < 1e-12
            && (f.scale_y - 1.0).abs() < 1e-12
            && f.rotation_deg.abs() < 1e-9
    );
    assert!(f.residuals_mm.iter().all(|r| *r < 1e-9));
}

#[test]
fn two_marks_of_a_board_turned_and_stretched_are_followed_exactly() {
    // turned by 2 degrees and stretched by 1.01 about the origin, then moved
    let t = 2.0_f64.to_radians();
    let (a, c) = (1.01 * t.cos(), 1.01 * t.sin());
    let make = |p: (f64, f64)| (a * p.0 - c * p.1 + 3.0, c * p.0 + a * p.1 - 1.0);
    let marks = [(0.0, 0.0), (40.0, 5.0), (7.0, 30.0)];
    let pairs: Vec<Pair> = marks.iter().map(|m| pair(*m, make(*m))).collect();
    let f = fit(&pairs[..2], Model::Similarity, &Limits::default()).unwrap();
    assert!(
        (f.rotation_deg - 2.0).abs() < 1e-9
            && (f.scale_x - 1.01).abs() < 1e-9
            && (f.scale_y - 1.01).abs() < 1e-9
    );
    assert!(
        close(f.affine.apply(marks[2]), make(marks[2])),
        "a mark that was not used lands where it was measured"
    );
}

#[test]
fn three_marks_give_the_exact_affine_transformation_and_four_marks_a_least_squares_fit() {
    let truth = Affine {
        a: 1.02,
        b: 0.01,
        c: -0.005,
        d: 0.99,
        e: 2.0,
        f: -3.0,
    };
    let marks = [(0.0, 0.0), (50.0, 0.0), (0.0, 40.0), (50.0, 40.0)];
    let pairs: Vec<Pair> = marks.iter().map(|m| pair(*m, truth.apply(*m))).collect();
    for n in [3, 4] {
        let f = fit(&pairs[..n], Model::Affine, &Limits::default()).unwrap();
        for (got, want) in [
            (f.affine.a, truth.a),
            (f.affine.b, truth.b),
            (f.affine.c, truth.c),
            (f.affine.d, truth.d),
            (f.affine.e, truth.e),
            (f.affine.f, truth.f),
        ] {
            assert!((got - want).abs() < 1e-9, "{n} marks: {got} against {want}");
        }
    }
    // one mark measured 0.05 mm off: the fit splits the error and the residuals show it
    let mut noisy = pairs.clone();
    noisy[3].measured.0 += 0.05;
    let f = fit(&noisy, Model::Affine, &Limits::default()).unwrap();
    assert!(
        f.residuals_mm.iter().all(|r| *r > 0.0 && *r < 0.05),
        "{:?}",
        f.residuals_mm
    );
}

#[test]
fn the_automatic_model_is_a_similarity_for_two_marks_and_an_affine_for_more() {
    // three marks with a shear: only an affine fit can reach them all
    let marks = [(0.0, 0.0), (50.0, 0.0), (0.0, 40.0)];
    let sheared = |p: (f64, f64)| (p.0 + 0.02 * p.1, p.1);
    let pairs: Vec<Pair> = marks.iter().map(|m| pair(*m, sheared(*m))).collect();
    let auto = fit(
        &pairs,
        Model::Auto,
        &Limits {
            max_residual_mm: 1e-6,
            ..Limits::default()
        },
    )
    .unwrap();
    assert!(auto.residuals_mm.iter().all(|r| *r < 1e-9));
    assert!(fit(
        &pairs,
        Model::Similarity,
        &Limits {
            max_residual_mm: 1e-6,
            ..Limits::default()
        }
    )
    .is_err());
    assert_eq!(Model::parse(" AFFINE ").unwrap(), Model::Affine);
    assert!(Model::parse("projective")
        .unwrap_err()
        .contains("projective"));
}

#[test]
fn marks_that_cannot_give_a_fit_are_refused_with_the_reason() {
    let lim = Limits::default();
    assert!(fit(&[], Model::Auto, &lim)
        .unwrap_err()
        .contains("at least 2"));
    assert!(fit(&[pair((0.0, 0.0), (0.0, 0.0))], Model::Auto, &lim)
        .unwrap_err()
        .contains("1 reference points"));
    let two = [pair((0.0, 0.0), (0.0, 0.0)), pair((10.0, 0.0), (10.0, 0.0))];
    assert!(fit(&two, Model::Affine, &lim)
        .unwrap_err()
        .contains("at least 3"));
    let same = [pair((5.0, 5.0), (5.0, 5.0)), pair((5.0, 5.0), (5.1, 5.0))];
    assert!(fit(&same, Model::Auto, &lim)
        .unwrap_err()
        .contains("same place"));
    let nan = [
        pair((0.0, 0.0), (0.0, 0.0)),
        pair((f64::NAN, 0.0), (1.0, 0.0)),
    ];
    assert!(fit(&nan, Model::Auto, &lim)
        .unwrap_err()
        .contains("fiducial 2"));
    // three marks on a line: an affine fit is impossible, a similarity is fine
    let line: Vec<Pair> = (0..3)
        .map(|i| pair((f64::from(i) * 10.0, 0.0), (f64::from(i) * 10.0 + 1.0, 0.5)))
        .collect();
    assert!(fit(&line, Model::Affine, &lim)
        .unwrap_err()
        .contains("line"));
    assert!(fit(&line, Model::Similarity, &lim).is_ok());
}

#[test]
fn a_fit_that_would_turn_the_board_over_is_refused() {
    let flipped = [
        pair((0.0, 0.0), (0.0, 0.0)),
        pair((10.0, 0.0), (-10.0, 0.0)),
        pair((0.0, 10.0), (0.0, 10.0)),
    ];
    let e = fit(
        &flipped,
        Model::Affine,
        &Limits {
            max_scale_deviation: 10.0,
            max_residual_mm: 10.0,
        },
    )
    .unwrap_err();
    assert!(e.contains("determinant"), "{e}");
}

#[test]
fn the_limits_of_scale_and_residual_are_exact() {
    let stretched = |k: f64| {
        [
            pair((0.0, 0.0), (0.0, 0.0)),
            pair((100.0, 0.0), (100.0 * k, 0.0)),
        ]
    };
    assert!(fit(&stretched(1.049), Model::Similarity, &Limits::default()).is_ok());
    let e = fit(&stretched(1.06), Model::Similarity, &Limits::default()).unwrap_err();
    assert!(
        e.contains("scale of x") && e.contains("fiducial_max_scale_deviation"),
        "{e}"
    );
    assert!(fit(
        &stretched(1.06),
        Model::Similarity,
        &Limits {
            max_scale_deviation: 0.1,
            ..Limits::default()
        }
    )
    .is_ok());
    let bent = [
        pair((0.0, 0.0), (0.0, 0.0)),
        pair((50.0, 0.0), (50.0, 0.0)),
        pair((0.0, 40.0), (0.0, 40.0)),
        pair((50.0, 40.0), (52.0, 40.0)),
    ];
    let e = fit(&bent, Model::Affine, &Limits::default()).unwrap_err();
    assert!(e.contains("fiducial_max_residual_mm"), "{e}");
    assert!(fit(
        &bent,
        Model::Affine,
        &Limits {
            max_residual_mm: 1.0,
            ..Limits::default()
        }
    )
    .is_ok());
}

#[test]
fn transformations_compose_in_the_order_they_are_applied() {
    let a = Affine {
        a: 1.0,
        b: 0.1,
        c: 0.0,
        d: 1.0,
        e: 5.0,
        f: 0.0,
    };
    let b = Affine {
        a: 2.0,
        b: 0.0,
        c: 0.3,
        d: 1.0,
        e: 0.0,
        f: -4.0,
    };
    for p in [(0.0, 0.0), (3.0, -7.0), (100.5, 20.25)] {
        assert!(close(a.then(&b).apply(p), b.apply(a.apply(p))));
    }
    assert!(Affine::identity().is_identity() && !a.is_identity());
    assert!(close(
        Affine::identity().then(&a).apply((3.0, 4.0)),
        a.apply((3.0, 4.0))
    ));
    assert!(Affine::scale(1.0, 1.0).validate().is_ok());
    assert!(Affine::scale(0.0, 1.0).validate().is_err());
    assert!(Affine::scale(-1.0, 1.0).validate().is_err());
    assert!(Affine {
        e: f64::INFINITY,
        ..Affine::identity()
    }
    .validate()
    .is_err());
}

#[test]
fn a_mark_is_four_numbers_with_or_without_units() {
    let p = parse_pair("10, 20; 10.5 20.25", Unit::Mm).unwrap();
    assert_eq!((p.design, p.measured), ((10.0, 20.0), (10.5, 20.25)));
    let q = parse_pair("1in 0 1in 1mil", Unit::Mm).unwrap();
    assert!(close(q.design, (25.4, 0.0)) && close(q.measured, (25.4, 0.0254)));
    let bare_inches = parse_pair("1 1 2 2", Unit::Inch).unwrap();
    assert!(close(bare_inches.measured, (50.8, 50.8)));
    for bad in ["", "1 2 3", "1 2 3 4 5", "1 2 3 x", "1 2 3 inf", "1,,2,3"] {
        assert!(parse_pair(bad, Unit::Mm).is_err(), "{bad:?}");
    }
}

#[test]
fn moving_the_passes_moves_every_point_and_scales_the_offset_and_a_bad_transform_changes_nothing() {
    let pass = Pass {
        offset_mm: 0.1,
        paths: vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]],
        open: vec![vec![(1.0, 1.0), (2.0, 2.0)]],
        ..Pass::default()
    };
    let mut passes = vec![pass.clone()];
    let t = Affine {
        a: 2.0,
        b: 0.0,
        c: 0.0,
        d: 2.0,
        e: 1.0,
        f: -1.0,
    };
    apply_to_passes(&mut passes, &t).unwrap();
    assert_eq!(passes[0].paths[0][1], (21.0, -1.0));
    assert_eq!(passes[0].open[0][0], (3.0, 1.0));
    assert!((passes[0].offset_mm - 0.2).abs() < 1e-12);
    let mut untouched = vec![pass.clone()];
    assert!(apply_to_passes(&mut untouched, &Affine::scale(-1.0, 1.0)).is_err());
    assert_eq!(untouched, vec![pass]);
    let mut huge = vec![Pass {
        paths: vec![vec![(f64::MAX, 0.0), (1.0, 1.0), (2.0, 2.0)]],
        ..Pass::default()
    }];
    assert!(apply_to_passes(&mut huge, &Affine::scale(10.0, 10.0))
        .unwrap_err()
        .contains("no longer a number"));
}

#[test]
fn moving_the_holes_keeps_their_diameter_and_tool() {
    let mut holes = vec![Hole {
        tool: 3,
        x_mm: 5.0,
        y_mm: 6.0,
        diameter_mm: 0.8,
    }];
    apply_to_holes(
        &mut holes,
        &Affine {
            e: 1.0,
            f: 2.0,
            ..Affine::identity()
        },
    )
    .unwrap();
    assert_eq!(
        holes[0],
        Hole {
            tool: 3,
            x_mm: 6.0,
            y_mm: 8.0,
            diameter_mm: 0.8
        }
    );
    assert!(apply_to_holes(&mut holes, &Affine::scale(0.0, 0.0)).is_err());
    assert_eq!(holes[0].x_mm, 6.0);
}

// ---- the order of the cuts -------------------------------------------------------------------------------------------------------------------------------------

fn cuts() -> Vec<Cut> {
    vec![
        square(50.0, 0.0, 2.0),
        square(0.0, 30.0, 2.0),
        square(10.0, 1.0, 2.0),
        Cut {
            pts: vec![(20.0, 20.0), (25.0, 20.0)],
            closed: false,
        },
        square(52.0, 30.0, 2.0),
        square(30.0, 5.0, 2.0),
    ]
}

fn key(c: &Cut) -> String {
    let mut p: Vec<String> = c
        .pts
        .iter()
        .map(|q| format!("{:.6},{:.6}", q.0, q.1))
        .collect();
    p.sort();
    format!("{}:{}", c.closed, p.join("|"))
}

#[test]
fn arranging_keeps_every_cut_once_whatever_the_strategy_and_the_start() {
    let mut want: Vec<String> = cuts().iter().map(key).collect();
    want.sort();
    for strategy in [Strategy::Original, Strategy::Nearest, Strategy::Optimized] {
        for loop_start in [
            LoopStart::Nearest,
            LoopStart::LowerLeft,
            LoopStart::Original,
        ] {
            for direction in [
                Direction::Climb,
                Direction::Conventional,
                Direction::Clockwise,
                Direction::CounterClockwise,
            ] {
                let o = Order {
                    strategy,
                    loop_start,
                    direction,
                    ..Order::default()
                };
                let (out, _) = arrange(&cuts(), &o, (7.0, 7.0));
                let mut got: Vec<String> = out.iter().map(key).collect();
                got.sort();
                assert_eq!(got, want, "{strategy:?} {loop_start:?} {direction:?}");
            }
        }
    }
}

#[test]
fn the_optimised_order_never_travels_more_than_the_nearest_one() {
    let near = arrange(
        &cuts(),
        &Order {
            strategy: Strategy::Nearest,
            ..Order::default()
        },
        (0.0, 0.0),
    )
    .0;
    let opt = arrange(
        &cuts(),
        &Order {
            strategy: Strategy::Optimized,
            ..Order::default()
        },
        (0.0, 0.0),
    )
    .0;
    assert!(travel_mm(&opt, (0.0, 0.0)) <= travel_mm(&near, (0.0, 0.0)) + 1e-9);
    assert_eq!(opt.len(), near.len());
}

#[test]
fn the_same_cuts_in_gives_the_same_cuts_out() {
    let o = Order {
        strategy: Strategy::Optimized,
        ..Order::default()
    };
    assert_eq!(
        arrange(&cuts(), &o, (3.0, 3.0)),
        arrange(&cuts(), &o, (3.0, 3.0))
    );
}

#[test]
fn the_direction_forces_the_way_round_and_leaves_open_runs_alone() {
    let mut mixed = vec![square(0.0, 0.0, 2.0), square(10.0, 0.0, 2.0)];
    mixed[1].pts.reverse();
    mixed.push(Cut {
        pts: vec![(30.0, 0.0), (35.0, 1.0)],
        closed: false,
    });
    for (d, want_cw, want_ccw) in [
        (Direction::Clockwise, 2, 0),
        (Direction::CounterClockwise, 0, 2),
    ] {
        let (out, _) = arrange(
            &mixed,
            &Order {
                direction: d,
                ..Order::default()
            },
            (0.0, 0.0),
        );
        assert_eq!(directions(&out), (want_cw, want_ccw, 1), "{d:?}");
        let open = out.iter().find(|c| !c.closed).unwrap();
        assert_eq!(
            open.pts,
            vec![(30.0, 0.0), (35.0, 1.0)],
            "an open run keeps its way"
        );
    }
    // conventional is climb turned round
    let climb = arrange(
        &mixed,
        &Order {
            direction: Direction::Climb,
            strategy: Strategy::Original,
            loop_start: LoopStart::Original,
            ..Order::default()
        },
        (0.0, 0.0),
    )
    .0;
    let conv = arrange(
        &mixed,
        &Order {
            direction: Direction::Conventional,
            strategy: Strategy::Original,
            loop_start: LoopStart::Original,
            ..Order::default()
        },
        (0.0, 0.0),
    )
    .0;
    for (a, b) in climb.iter().zip(&conv) {
        let mut r = a.pts.clone();
        r.reverse();
        assert_eq!(r, b.pts);
    }
    assert!(signed_area(&climb[0].pts) > 0.0 && signed_area(&conv[0].pts) < 0.0);
}

#[test]
fn a_closed_path_starts_at_its_lower_left_point_or_nearest_to_the_tool() {
    let c = vec![Cut {
        pts: vec![(5.0, 5.0), (9.0, 1.0), (7.0, 1.0), (3.0, 8.0)],
        closed: true,
    }];
    let (out, _) = arrange(
        &c,
        &Order {
            loop_start: LoopStart::LowerLeft,
            ..Order::default()
        },
        (100.0, 100.0),
    );
    assert_eq!(
        out[0].pts[0],
        (7.0, 1.0),
        "the lowest point, and the leftmost of two that are level"
    );
    let (out, end) = arrange(
        &c,
        &Order {
            loop_start: LoopStart::Nearest,
            ..Order::default()
        },
        (4.0, 9.0),
    );
    assert_eq!(out[0].pts[0], (3.0, 8.0));
    assert_eq!(end, (3.0, 8.0), "a closed path ends where it started");
}

#[test]
fn cuts_that_cannot_be_cut_are_left_out_and_nothing_gives_the_start_back() {
    let bad = vec![
        Cut {
            pts: vec![(0.0, 0.0), (1.0, 1.0)],
            closed: true,
        },
        Cut {
            pts: vec![(0.0, 0.0)],
            closed: false,
        },
        Cut {
            pts: vec![(0.0, 0.0), (f64::NAN, 1.0), (2.0, 2.0)],
            closed: true,
        },
        Cut {
            pts: vec![],
            closed: false,
        },
    ];
    let (out, end) = arrange(&bad, &Order::default(), (4.0, 5.0));
    assert!(out.is_empty());
    assert_eq!(end, (4.0, 5.0));
    assert_eq!(travel_mm(&[], (1.0, 1.0)), 0.0);
    assert_eq!(directions(&[]), (0, 0, 0));
}

#[test]
fn the_length_of_a_cut_counts_the_closing_side_only_when_it_is_closed() {
    let pts = vec![(0.0, 0.0), (3.0, 0.0), (3.0, 4.0)];
    assert!(
        (Cut {
            pts: pts.clone(),
            closed: false
        }
        .length_mm()
            - 7.0)
            .abs()
            < 1e-12
    );
    assert!((Cut { pts, closed: true }.length_mm() - 12.0).abs() < 1e-12);
}

// ---- lighter paths -----------------------------------------------------------------------------------------------------------------------------------------------

#[test]
fn repeated_points_are_joined_and_a_closed_path_does_not_repeat_its_first_point() {
    let o = Optimize::default();
    let (v, spikes) = optimize_path(
        &[
            (0.0, 0.0),
            (0.0, 0.0),
            (5.0, 0.0),
            (5.0, 0.0),
            (5.0, 5.0),
            (0.0, 0.0),
        ],
        true,
        &o,
    )
    .unwrap();
    assert_eq!((v, spikes), (vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)], 0));
    // an open run keeps both ends even if they coincide
    let (v, _) = optimize_path(&[(0.0, 0.0), (5.0, 0.0), (0.0, 0.0)], false, &o).unwrap();
    assert_eq!(v.len(), 3);
}

#[test]
fn a_spike_is_taken_out_only_when_it_is_short_and_turns_back() {
    let o = Optimize {
        spike_max_mm: 0.1,
        ..Optimize::default()
    };
    // out 0.05 mm and straight back along the same line
    let spiky = [
        (0.0, 0.0),
        (5.0, 0.0),
        (5.05, 0.0),
        (5.0, 0.0001),
        (10.0, 0.0),
    ];
    let (v, spikes) = optimize_path(&spiky, false, &o).unwrap();
    assert!(spikes >= 1 && v.len() < spiky.len(), "{v:?}");
    assert_eq!(
        (*v.first().unwrap(), *v.last().unwrap()),
        ((0.0, 0.0), (10.0, 0.0)),
        "the ends stay"
    );
    // a corner of 90 degrees is not a spike however short the side
    let corner = [(0.0, 0.0), (5.0, 0.0), (5.0, 0.05), (10.0, 0.05)];
    assert_eq!(optimize_path(&corner, false, &o).unwrap().1, 0);
    // a long way out and back is a real detour
    let detour = [
        (0.0, 0.0),
        (5.0, 0.0),
        (8.0, 0.0),
        (5.0, 0.001),
        (10.0, 0.0),
    ];
    assert_eq!(optimize_path(&detour, false, &o).unwrap().1, 0);
}

#[test]
fn simplifying_never_moves_a_point_farther_than_the_tolerance() {
    let tol = 0.02;
    let o = Optimize {
        tolerance_mm: tol,
        ..Optimize::default()
    };
    let wave: Vec<(f64, f64)> = (0..=200)
        .map(|i| (f64::from(i) * 0.1, (f64::from(i) * 0.3).sin() * 0.01))
        .collect();
    let (v, _) = optimize_path(&wave, false, &o).unwrap();
    assert!(v.len() < wave.len());
    assert_eq!((v[0], *v.last().unwrap()), (wave[0], *wave.last().unwrap()));
    // every original point is within the tolerance of a segment of the result
    for p in &wave {
        let d = v
            .windows(2)
            .map(|w| {
                let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                let l2 = dx * dx + dy * dy;
                let t = if l2 == 0.0 {
                    0.0
                } else {
                    (((p.0 - w[0].0) * dx + (p.1 - w[0].1) * dy) / l2).clamp(0.0, 1.0)
                };
                (p.0 - (w[0].0 + t * dx)).hypot(p.1 - (w[0].1 + t * dy))
            })
            .fold(f64::INFINITY, f64::min);
        assert!(d <= tol + 1e-9, "{p:?} is {d} away");
    }
}

#[test]
fn a_path_that_collapses_is_dropped_and_counted_and_a_bad_point_names_its_number() {
    let mut passes = vec![Pass {
        paths: vec![
            vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
            vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
        ],
        open: vec![vec![(1.0, 1.0), (1.0, 1.0)]],
        ..Pass::default()
    }];
    let stats = optimize_passes(
        &mut passes,
        &Optimize {
            tolerance_mm: 0.01,
            ..Optimize::default()
        },
    )
    .unwrap();
    assert_eq!(stats.dropped_paths, 2, "{stats:?}");
    assert_eq!(passes[0].paths.len(), 1);
    assert!(passes[0].open.is_empty());
    assert_eq!((stats.points_before, stats.points_after), (9, 4));
    let e = optimize_path(&[(0.0, 0.0), (1.0, f64::NAN)], false, &Optimize::default()).unwrap_err();
    assert!(e.contains("point 2"), "{e}");
    let mut bad = vec![Pass {
        paths: vec![vec![(0.0, 0.0), (1.0, 1.0), (f64::INFINITY, 0.0)]],
        ..Pass::default()
    }];
    assert!(optimize_passes(&mut bad, &Optimize::default())
        .unwrap_err()
        .starts_with("pass 1"));
}

#[test]
fn the_optimising_settings_are_valid_exactly_inside_their_ranges() {
    let ok = |o: Optimize| o.validate().is_ok();
    assert!(ok(Optimize::default()) && Optimize::default().is_off());
    assert!(
        ok(Optimize {
            tolerance_mm: 1.0,
            ..Optimize::default()
        }) && !ok(Optimize {
            tolerance_mm: 1.01,
            ..Optimize::default()
        })
    );
    assert!(
        !ok(Optimize {
            tolerance_mm: -0.001,
            ..Optimize::default()
        }) && !ok(Optimize {
            tolerance_mm: f64::NAN,
            ..Optimize::default()
        })
    );
    assert!(
        ok(Optimize {
            spike_max_mm: 5.0,
            ..Optimize::default()
        }) && !ok(Optimize {
            spike_max_mm: 5.1,
            ..Optimize::default()
        })
    );
    assert!(
        ok(Optimize {
            spike_angle_deg: 90.0,
            ..Optimize::default()
        }) && ok(Optimize {
            spike_angle_deg: 180.0,
            ..Optimize::default()
        })
    );
    assert!(!ok(Optimize {
        spike_angle_deg: 89.9,
        ..Optimize::default()
    }));
    assert!(!Optimize {
        tolerance_mm: 0.01,
        ..Optimize::default()
    }
    .is_off());
}

// ---- turning, mirroring and laying out ----------------------------------------------------------------------------------------------------------------

fn scene() -> Scene {
    Scene {
        shapes: vec![
            Shape::Disc {
                x: 1.0,
                y: 2.0,
                r: 0.5,
            },
            Shape::Rect {
                x: 5.0,
                y: 3.0,
                w: 2.0,
                h: 1.0,
            },
            Shape::Obround {
                x: 8.0,
                y: 1.0,
                w: 3.0,
                h: 1.0,
            },
            Shape::Stroke {
                x0: 0.0,
                y0: 0.0,
                x1: 9.0,
                y1: 4.0,
                w: 0.2,
            },
            Shape::Region {
                pts: vec![(2.0, 2.0), (4.0, 2.0), (3.0, 5.0)],
            },
        ],
        notes: vec!["a note".into()],
    }
}

#[test]
fn mirroring_twice_on_the_same_line_gives_the_scene_back_and_keeps_the_notes() {
    for (axis, at) in [
        (Axis::Vertical, 4.5),
        (Axis::Horizontal, -2.0),
        (Axis::Vertical, 0.0),
    ] {
        let once = mirror_scene(&scene(), axis, at);
        assert_ne!(once.shapes, scene().shapes);
        assert_eq!(mirror_scene(&once, axis, at), scene(), "{axis:?} {at}");
        assert_eq!(once.notes, vec!["a note".to_string()]);
    }
    let holes = vec![Hole {
        tool: 1,
        x_mm: 3.0,
        y_mm: 4.0,
        diameter_mm: 1.0,
    }];
    assert_eq!(
        mirror_holes(
            &mirror_holes(&holes, Axis::Vertical, 10.0),
            Axis::Vertical,
            10.0
        ),
        holes
    );
    assert_eq!(
        mirror_holes(&holes, Axis::Horizontal, 4.0)[0].y_mm,
        4.0,
        "a point on the line stays"
    );
}

#[test]
fn the_middle_line_of_a_board_is_the_middle_of_its_box() {
    let b = bounds(2.0, 10.0, 12.0, 30.0);
    assert_eq!(Axis::Vertical.middle(&b), 7.0);
    assert_eq!(Axis::Horizontal.middle(&b), 20.0);
    assert_eq!(Axis::parse_option("NONE").unwrap(), None);
    assert_eq!(
        Axis::parse_option(" Vertical ").unwrap(),
        Some(Axis::Vertical)
    );
    assert!(Axis::parse_option("diagonal").is_err());
}

#[test]
fn four_quarter_turns_are_no_turn_and_a_turn_and_its_complement_cancel() {
    let c = (4.0, 2.5);
    assert_eq!(
        rotate_scene(
            &rotate_scene(&rotate_scene(&rotate_scene(&scene(), 1, c), 1, c), 1, c),
            1,
            c
        ),
        scene()
    );
    assert_eq!(rotate_scene(&rotate_scene(&scene(), 1, c), 3, c), scene());
    assert_eq!(rotate_scene(&rotate_scene(&scene(), 2, c), 2, c), scene());
    assert_eq!(rotate_scene(&scene(), 0, c), scene());
    assert_eq!(
        rotate_scene(&scene(), 4, c),
        scene(),
        "four quarters is the same as none"
    );
    let holes = vec![Hole {
        tool: 2,
        x_mm: 3.0,
        y_mm: 4.0,
        diameter_mm: 0.9,
    }];
    assert_eq!(rotate_holes(&rotate_holes(&holes, 1, c), 3, c), holes);
}

#[test]
fn a_quarter_turn_counterclockwise_sends_east_to_north_and_swaps_width_and_height() {
    let s = Scene {
        shapes: vec![
            Shape::Rect {
                x: 10.0,
                y: 0.0,
                w: 4.0,
                h: 1.0,
            },
            Shape::Obround {
                x: 10.0,
                y: 0.0,
                w: 4.0,
                h: 1.0,
            },
        ],
        notes: vec![],
    };
    let r = rotate_scene(&s, 1, (0.0, 0.0));
    assert_eq!(
        r.shapes[0],
        Shape::Rect {
            x: 0.0,
            y: 10.0,
            w: 1.0,
            h: 4.0
        }
    );
    assert_eq!(
        r.shapes[1],
        Shape::Obround {
            x: 0.0,
            y: 10.0,
            w: 1.0,
            h: 4.0
        }
    );
    let half = rotate_scene(&s, 2, (0.0, 0.0));
    assert_eq!(
        half.shapes[0],
        Shape::Rect {
            x: -10.0,
            y: 0.0,
            w: 4.0,
            h: 1.0
        }
    );
}

#[test]
fn a_panel_has_the_first_copy_in_place_and_the_others_a_board_and_a_gap_apart() {
    let b = bounds(0.0, 0.0, 10.0, 5.0);
    let o = panel_offsets(&b, 3, 2, 1.0).unwrap();
    assert_eq!(o.len(), 6);
    assert_eq!(o[0], (0.0, 0.0));
    assert_eq!(o[1], (11.0, 0.0));
    assert_eq!(o[3], (0.0, 6.0));
    assert_eq!(o[5], (22.0, 6.0));
    for (cols, rows, gap) in [
        (0, 1, 1.0),
        (1, 0, 1.0),
        (11, 10, 1.0),
        (1, 1, -0.1),
        (1, 1, f64::NAN),
    ] {
        assert!(
            panel_offsets(&b, cols, rows, gap).is_err(),
            "{cols} x {rows}, gap {gap}"
        );
    }
    assert!(
        panel_offsets(&b, 10, 10, 0.0).is_ok(),
        "a hundred boards is the largest panel"
    );
    assert!(panel_offsets(&Bounds::empty(), 1, 1, 0.0)
        .unwrap_err()
        .contains("empty"));
}

#[test]
fn laying_out_a_board_keeps_the_copper_the_holes_and_the_outline_together() {
    let board = Board {
        scene: Scene {
            shapes: vec![Shape::Disc {
                x: 1.0,
                y: 1.0,
                r: 0.5,
            }],
            notes: vec![],
        },
        holes: vec![Hole {
            tool: 1,
            x_mm: 2.0,
            y_mm: 1.0,
            diameter_mm: 0.8,
        }],
        outline: vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 4.0), (0.0, 4.0)]],
    };
    let (out, whole) = lay_out(
        &board,
        &Layout {
            cols: 2,
            rows: 2,
            gap_mm: 2.0,
            ..Layout::default()
        },
    )
    .unwrap();
    assert_eq!(
        (out.scene.shapes.len(), out.holes.len(), out.outline.len()),
        (4, 4, 4)
    );
    assert!(
        (whole.width() - 22.0).abs() < 1e-9 && (whole.height() - 10.0).abs() < 1e-9,
        "{whole:?}"
    );
    // every copy has the hole at the same place inside its own outline
    for (h, o) in out.holes.iter().zip(&out.outline) {
        assert!(close((h.x_mm - o[0].0, h.y_mm - o[0].1), (2.0, 1.0)));
    }
    // a quarter turn about the middle of the outline, then a mirror: the box of a 10 x 4 board is 4 x 10 whatever follows
    let (turned, box_) = lay_out(
        &board,
        &Layout {
            quarters: 1,
            mirror: Some(Axis::Vertical),
            ..Layout::default()
        },
    )
    .unwrap();
    assert!(
        (box_.width() - 4.0).abs() < 1e-9 && (box_.height() - 10.0).abs() < 1e-9,
        "{box_:?}"
    );
    assert_eq!(turned.holes.len(), 1);
    assert!(lay_out(
        &board,
        &Layout {
            quarters: 4,
            ..Layout::default()
        }
    )
    .is_err());
    assert!(lay_out(&Board::default(), &Layout::default())
        .unwrap_err()
        .contains("nothing"));
}

#[test]
fn the_reference_box_is_the_outline_then_the_copper_then_the_holes() {
    let mut b = Board {
        holes: vec![Hole {
            tool: 1,
            x_mm: 3.0,
            y_mm: 4.0,
            diameter_mm: 1.0,
        }],
        ..Board::default()
    };
    assert_eq!((b.reference().min_x, b.reference().max_y), (3.0, 4.0));
    b.scene = Scene {
        shapes: vec![Shape::Rect {
            x: 10.0,
            y: 10.0,
            w: 2.0,
            h: 2.0,
        }],
        notes: vec![],
    };
    assert_eq!(b.reference().max_x, 11.0);
    b.outline = vec![vec![(0.0, 0.0), (50.0, 0.0), (50.0, 20.0)]];
    assert_eq!(b.reference().max_x, 50.0);
}

#[test]
fn the_holes_that_hold_a_board_are_the_same_set_when_it_is_turned_over() {
    let b = bounds(0.0, 0.0, 60.0, 40.0);
    let cases = [
        (2usize, HolePattern::Normal, Axis::Vertical),
        (6, HolePattern::Normal, Axis::Vertical),
        (2, HolePattern::Normal, Axis::Horizontal),
        (6, HolePattern::Normal, Axis::Horizontal),
        (2, HolePattern::Special, Axis::Vertical),
        (6, HolePattern::Special, Axis::Vertical),
        (6, HolePattern::Special, Axis::Horizontal),
    ];
    for (count, pattern, axis) in cases {
        let holes = alignment_pattern(&b, 3.0, 5.0, count, axis, 9, pattern, (40.0, 60.0)).unwrap();
        assert_eq!(holes.len(), count);
        let mirrored = mirror_holes(&holes, axis, axis.middle(&b));
        let set = |h: &[Hole]| {
            let mut v: Vec<String> = h
                .iter()
                .map(|x| format!("{:.6},{:.6}", x.x_mm, x.y_mm))
                .collect();
            v.sort();
            v
        };
        assert_eq!(set(&holes), set(&mirrored), "{count} {pattern:?} {axis:?}");
        assert!(holes.iter().all(|h| h.tool == 9 && h.diameter_mm == 3.0));
    }
}

#[test]
fn the_third_hole_is_what_tells_a_board_from_the_same_board_turned_half_a_turn() {
    let b = bounds(0.0, 0.0, 60.0, 40.0);
    let three = alignment_holes(&b, 3.0, 5.0, 3, Axis::Vertical, 1).unwrap();
    assert_eq!(three.len(), 3);
    // half a turn about the middle of the board must not give the same set
    let turned = rotate_holes(&three, 2, b.center());
    let set = |h: &[Hole]| {
        let mut v: Vec<String> = h
            .iter()
            .map(|x| format!("{:.6},{:.6}", x.x_mm, x.y_mm))
            .collect();
        v.sort();
        v
    };
    assert_ne!(set(&three), set(&turned));
    let two = alignment_holes(&b, 3.0, 5.0, 2, Axis::Vertical, 1).unwrap();
    assert_eq!(
        set(&two),
        set(&rotate_holes(&two, 2, b.center())),
        "two holes are the same after half a turn: that is why there is a third"
    );
}

#[test]
fn the_alignment_holes_refuse_numbers_that_make_no_sense() {
    let b = bounds(0.0, 0.0, 60.0, 40.0);
    let go = |d: f64, m: f64, n: usize, p: HolePattern, s: (f64, f64)| {
        alignment_pattern(&b, d, m, n, Axis::Vertical, 1, p, s)
    };
    assert!(go(3.0, 5.0, 4, HolePattern::Normal, (40.0, 60.0))
        .unwrap_err()
        .contains("2, 3 or 6"));
    assert!(go(0.0, 5.0, 2, HolePattern::Normal, (40.0, 60.0)).is_err());
    assert!(go(3.0, -1.0, 2, HolePattern::Normal, (40.0, 60.0)).is_err());
    assert!(go(f64::NAN, 5.0, 2, HolePattern::Normal, (40.0, 60.0)).is_err());
    assert!(go(3.0, 5.0, 3, HolePattern::Special, (40.0, 60.0))
        .unwrap_err()
        .contains("2 or 6"));
    assert!(go(3.0, 5.0, 2, HolePattern::Special, (0.0, 60.0)).is_err());
    assert!(
        alignment_holes(&Bounds::empty(), 3.0, 5.0, 2, Axis::Vertical, 1)
            .unwrap_err()
            .contains("no board")
    );
    assert_eq!(HolePattern::parse("S").unwrap(), HolePattern::Special);
    assert!(HolePattern::parse("odd").is_err());
}
