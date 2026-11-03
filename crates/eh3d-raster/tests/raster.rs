use eh3d_raster::contour::{loops, orient_climb, simplify, simplify_open, Grid};
use eh3d_raster::edt::{distance_mm, squared_distance};
use eh3d_raster::units::{parse_length, Unit};

#[test]
fn units_convert_both_ways() {
    assert_eq!(Unit::parse("MIL").unwrap(), Unit::Mil);
    assert_eq!(Unit::parse("in").unwrap(), Unit::Inch);
    assert!(Unit::parse("furlong").is_err());
    assert!((Unit::Inch.to_mm(1.0) - 25.4).abs() < 1e-12);
    assert!((Unit::Mil.from_mm(0.0254) - 1.0).abs() < 1e-12);
    assert_eq!(Unit::Mm.name(), "mm");
}

#[test]
fn lengths_take_their_own_unit_or_the_bare_one() {
    assert!((parse_length("8mil", Unit::Mm).unwrap() - 0.2032).abs() < 1e-9);
    assert!((parse_length("0.01in", Unit::Mm).unwrap() - 0.254).abs() < 1e-9);
    assert!((parse_length("0.2", Unit::Mm).unwrap() - 0.2).abs() < 1e-12);
    assert!((parse_length("8", Unit::Mil).unwrap() - 0.2032).abs() < 1e-9);
    assert!((parse_length("8 mils", Unit::Mm).unwrap() - 0.2032).abs() < 1e-9);
    assert!(parse_length("", Unit::Mm).is_err());
    assert!(parse_length("abc", Unit::Mm).is_err());
    assert!(parse_length("inf", Unit::Mm).is_err());
}

#[test]
fn distance_is_exact_euclidean() {
    let (w, h) = (9, 7);
    let mut on = vec![false; w * h];
    on[3 * w + 4] = true;
    let d = squared_distance(&on, w, h);
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f32 - 4.0, y as f32 - 3.0);
            assert_eq!(d[y * w + x], dx * dx + dy * dy);
        }
    }
    let mm = distance_mm(&on, w, h, 0.5);
    assert!((mm[3 * w + 7] - 1.5).abs() < 1e-6);
}

#[test]
fn distance_without_copper_is_huge() {
    let d = distance_mm(&[false; 12], 4, 3, 0.1);
    assert!(d.iter().all(|&v| v == f32::MAX));
}

fn disc_grid(w: usize, h: usize, res: f64) -> (Vec<f32>, usize, usize) {
    let mut on = vec![false; w * h];
    on[(h / 2) * w + w / 2] = true;
    (distance_mm(&on, w, h, res), w, h)
}

#[test]
fn contour_of_a_point_is_a_closed_circle() {
    let (dist, w, h) = disc_grid(41, 41, 0.1);
    let g = Grid {
        dist: &dist,
        w,
        h,
        res: 0.1,
        x0: 0.0,
        y0: 0.0,
    };
    let found = loops(&g, 1.0);
    assert_eq!(found.len(), 1);
    let centre = (20.5 * 0.1, 20.5 * 0.1);
    for p in &found[0] {
        let r = (p.0 - centre.0).hypot(p.1 - centre.1);
        assert!((r - 1.0).abs() < 0.1, "radius {r}");
    }
}

#[test]
fn simplify_keeps_the_shape_within_the_tolerance() {
    let (dist, w, h) = disc_grid(41, 41, 0.1);
    let g = Grid {
        dist: &dist,
        w,
        h,
        res: 0.1,
        x0: 0.0,
        y0: 0.0,
    };
    let circle = loops(&g, 1.0).remove(0);
    let light = simplify(&circle, 0.02);
    assert!(light.len() < circle.len());
    assert!(light.len() >= 8);
    let line: Vec<(f64, f64)> = (0..10).map(|i| (f64::from(i), 0.0)).collect();
    assert_eq!(simplify_open(&line, 0.01), vec![(0.0, 0.0), (9.0, 0.0)]);
}

#[test]
fn the_tool_goes_clockwise_round_the_copper() {
    let (dist, w, h) = disc_grid(41, 41, 0.1);
    let g = Grid {
        dist: &dist,
        w,
        h,
        res: 0.1,
        x0: 0.0,
        y0: 0.0,
    };
    let mut path = loops(&g, 1.0).remove(0);
    orient_climb(&g, &mut path);
    let area: f64 = (0..path.len())
        .map(|i| {
            let (a, b) = (path[i], path[(i + 1) % path.len()]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        / 2.0;
    let first = path.clone();
    orient_climb(&g, &mut path);
    assert_eq!(first, path, "a second call must not turn it again");
    assert!(area.abs() > 1.0);
    let mut turned = path.clone();
    turned.reverse();
    orient_climb(&g, &mut turned);
    assert_eq!(turned, path);
}

#[test]
fn a_path_between_two_pieces_of_copper_goes_the_way_of_the_nearer_one() {
    // the copper is a point at (10, 5): the longest edge, along y = 5, is as far on one side as on the other
    let (res, w, h) = (0.05, 240usize, 240usize);
    let dist: Vec<f32> = (0..w * h)
        .map(|i| {
            let (x, y) = (((i % w) as f64 + 0.5) * res, ((i / w) as f64 + 0.5) * res);
            (x - 10.0).hypot(y - 5.0) as f32
        })
        .collect();
    let g = Grid {
        dist: &dist,
        w,
        h,
        res,
        x0: 0.0,
        y0: 0.0,
    };
    let path = vec![(2.0, 3.0), (2.0, 5.0), (8.0, 5.0), (8.0, 3.0)];
    let mut again = path.clone();
    orient_climb(&g, &mut again);
    assert_eq!(again, path, "the nearer sides are on its right: it stays");
    let mut turned: Vec<(f64, f64)> = path.iter().rev().cloned().collect();
    orient_climb(&g, &mut turned);
    assert_eq!(turned, path);
}
