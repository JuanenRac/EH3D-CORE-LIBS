//! The distance transform against a slow exact answer, the contours with more than one piece of copper and the units at their edges.

use eh3d_raster::contour::{loops, simplify, simplify_open, Grid};
use eh3d_raster::edt::{distance_mm, squared_distance};
use eh3d_raster::units::{parse_length, Unit};

/// A pattern of copper that depends only on the seed: the same every run.
fn pattern(w: usize, h: usize, seed: u64, density: u64) -> Vec<bool> {
    let mut s = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (0..w * h)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (s >> 33) % 100 < density
        })
        .collect()
}

fn brute(on: &[bool], w: usize, h: usize) -> Vec<f32> {
    (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as f32, (i / w) as f32);
            on.iter()
                .enumerate()
                .filter(|(_, &b)| b)
                .map(|(j, _)| {
                    let (a, b) = ((j % w) as f32, (j / w) as f32);
                    (x - a) * (x - a) + (y - b) * (y - b)
                })
                .fold(f32::INFINITY, f32::min)
        })
        .collect()
}

#[test]
fn the_distance_matches_the_slow_exact_answer_on_grids_of_every_shape() {
    for (w, h) in [
        (1, 1),
        (1, 9),
        (9, 1),
        (2, 2),
        (13, 5),
        (5, 13),
        (16, 16),
        (31, 7),
    ] {
        for (seed, density) in [(1, 3), (2, 10), (3, 40), (4, 90)] {
            let on = pattern(w, h, seed, density);
            if !on.iter().any(|&b| b) {
                continue;
            }
            let fast = squared_distance(&on, w, h);
            let slow = brute(&on, w, h);
            assert_eq!(fast, slow, "{w} x {h}, seed {seed}, density {density}");
        }
    }
}

#[test]
fn copper_everywhere_means_zero_everywhere_and_the_distance_is_zero_on_every_copper_cell() {
    assert!(squared_distance(&[true; 20], 5, 4)
        .iter()
        .all(|&d| d == 0.0));
    let on = pattern(12, 9, 7, 20);
    let d = squared_distance(&on, 12, 9);
    for (i, &b) in on.iter().enumerate() {
        assert_eq!(b, d[i] == 0.0, "cell {i}");
    }
}

#[test]
fn the_distance_in_millimetres_scales_with_the_cell_and_never_goes_negative_or_nan() {
    let on = pattern(10, 10, 5, 15);
    let a = distance_mm(&on, 10, 10, 0.05);
    let b = distance_mm(&on, 10, 10, 0.10);
    for (x, y) in a.iter().zip(&b) {
        assert!(x.is_finite() && *x >= 0.0);
        assert!((y - 2.0 * x).abs() <= 1e-5 * y.max(1.0));
    }
    assert!(distance_mm(&[], 0, 0, 1.0).is_empty());
}

fn two_points(gap: usize) -> (Vec<f32>, usize, usize) {
    let (w, h) = (60, 30);
    let mut on = vec![false; w * h];
    on[15 * w + 10] = true;
    on[15 * w + 10 + gap] = true;
    (distance_mm(&on, w, h, 0.1), w, h)
}

#[test]
fn two_pieces_of_copper_far_apart_give_two_circles_and_close_together_one_outline() {
    let apart = {
        let (dist, w, h) = two_points(30);
        loops(
            &Grid {
                dist: &dist,
                w,
                h,
                res: 0.1,
                x0: 0.0,
                y0: 0.0,
            },
            0.5,
        )
    };
    assert_eq!(apart.len(), 2);
    let near = {
        let (dist, w, h) = two_points(6);
        loops(
            &Grid {
                dist: &dist,
                w,
                h,
                res: 0.1,
                x0: 0.0,
                y0: 0.0,
            },
            0.5,
        )
    };
    assert_eq!(
        near.len(),
        1,
        "the two circles of radius 0.5 around points 0.6 apart are one outline"
    );
}

#[test]
fn a_grid_with_no_copper_has_no_contour_and_every_loop_is_closed_and_has_points() {
    let none = distance_mm(&[false; 400], 20, 20, 0.1);
    assert!(loops(
        &Grid {
            dist: &none,
            w: 20,
            h: 20,
            res: 0.1,
            x0: 0.0,
            y0: 0.0
        },
        0.5
    )
    .is_empty());
    let (dist, w, h) = two_points(20);
    for l in loops(
        &Grid {
            dist: &dist,
            w,
            h,
            res: 0.1,
            x0: 5.0,
            y0: -3.0,
        },
        0.7,
    ) {
        assert!(l.len() >= 3);
        // the origin of the grid moves the loop
        assert!(l.iter().all(|p| p.0 > 5.0 && p.1 < 0.0));
    }
}

#[test]
fn the_same_distances_give_the_same_loops_every_time() {
    let (dist, w, h) = two_points(12);
    let g = Grid {
        dist: &dist,
        w,
        h,
        res: 0.1,
        x0: 0.0,
        y0: 0.0,
    };
    assert_eq!(loops(&g, 0.6), loops(&g, 0.6));
}

#[test]
fn simplifying_never_moves_a_point_more_than_the_tolerance_and_keeps_the_ends_of_an_open_run() {
    let wave: Vec<(f64, f64)> = (0..100)
        .map(|i| (f64::from(i) * 0.1, (f64::from(i) * 0.2).sin() * 0.03))
        .collect();
    let tol = 0.01;
    let light = simplify_open(&wave, tol);
    assert!(light.len() < wave.len());
    assert_eq!(
        (light[0], *light.last().unwrap()),
        (wave[0], *wave.last().unwrap())
    );
    for p in &wave {
        let d = light
            .windows(2)
            .map(|w| {
                let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                let t = (((p.0 - w[0].0) * dx + (p.1 - w[0].1) * dy) / (dx * dx + dy * dy))
                    .clamp(0.0, 1.0);
                (p.0 - w[0].0 - t * dx).hypot(p.1 - w[0].1 - t * dy)
            })
            .fold(f64::INFINITY, f64::min);
        assert!(d <= tol + 1e-9, "{p:?} is {d} from the simplified path");
    }
    assert_eq!(
        simplify_open(&wave, 0.0).len(),
        wave.len(),
        "a tolerance of nothing keeps the points that are not on a straight line"
    );
    assert_eq!(simplify_open(&[(1.0, 1.0)], 0.1), vec![(1.0, 1.0)]);
    assert!(simplify_open(&[], 0.1).is_empty());
    assert!(simplify(&[], 0.1).is_empty());
}

#[test]
fn lengths_at_the_edges_of_what_can_be_written() {
    let mm = |t: &str| parse_length(t, Unit::Mm);
    assert!((mm("1e3mil").unwrap() - 25.4).abs() < 1e-9);
    assert!((mm("5 THOU").unwrap() - 0.127).abs() < 1e-12);
    assert!((mm("2INCHES").unwrap() - 50.8).abs() < 1e-9);
    assert!((mm("1\"").unwrap() - 25.4).abs() < 1e-12);
    assert!((mm("-0.5mm").unwrap() + 0.5).abs() < 1e-12);
    assert!((mm("  12.5   mm  ").unwrap() - 12.5).abs() < 1e-12);
    for bad in [
        "mm",
        "mil",
        "5 5mm",
        "--5mm",
        "5min",
        "1e400mm",
        "nanmm",
        "infin",
        "5 furlongs",
    ] {
        assert!(mm(bad).is_err(), "{bad:?}");
    }
    let e = mm("abc").unwrap_err();
    assert!(e.contains("abc"), "the message names the text: {e}");
}

#[test]
fn every_unit_comes_back_to_the_same_number() {
    for u in [Unit::Mm, Unit::Mil, Unit::Inch] {
        for v in [0.0, 1.0, -3.7, 12345.678] {
            assert!(
                (u.from_mm(u.to_mm(v)) - v).abs() < 1e-9 * v.abs().max(1.0),
                "{u:?} {v}"
            );
        }
        assert_eq!(
            Unit::parse(u.name()).unwrap(),
            u,
            "the name of a unit is a name it reads back"
        );
    }
    for (text, want) in [
        (" MM ", Unit::Mm),
        ("Millimetres", Unit::Mm),
        ("thou", Unit::Mil),
        ("Inches", Unit::Inch),
        ("\"", Unit::Inch),
    ] {
        assert_eq!(Unit::parse(text).unwrap(), want, "{text:?}");
    }
    assert!(Unit::parse("").is_err());
}
