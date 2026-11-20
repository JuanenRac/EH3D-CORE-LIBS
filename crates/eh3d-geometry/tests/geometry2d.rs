//! Properties of the 2D geometry (booleans, offsets, simplification and arc recovery) checked on shapes made
//! from a generator with a fixed seed, against oracles that do not use the code under test: the area of a union of
//! rectangles on a whole number grid counted cell by cell, the identities between the four operations, the formula of
//! the radius of a chain, and the distance of every point left out by the simplification.

use eh3d_geometry::arcfit::{radius_from_chain, recover_arcs, ArcRecovery, Piece};
use eh3d_geometry::boolean::{
    combine, difference, intersection, union, union_all, xor, BooleanOptions, Operation,
};
use eh3d_geometry::offset::{offset, stroke, OffsetOptions};
use eh3d_geometry::polygon::{signed_area, simplify, Containment, Pt, Region};

/// A small deterministic generator (SplitMix64): the same seed always gives the same numbers.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn opts() -> BooleanOptions {
    BooleanOptions::default()
}

/// A rectangle with whole number corners inside 0..=size.
fn random_rect(rng: &mut Seeded, size: u64) -> (i64, i64, i64, i64) {
    let x0 = rng.below(size) as i64;
    let y0 = rng.below(size) as i64;
    let w = 1 + rng.below(size / 2) as i64;
    let h = 1 + rng.below(size / 2) as i64;
    (x0, y0, x0 + w, y0 + h)
}

fn rect_region(r: (i64, i64, i64, i64)) -> Region {
    Region::rectangle(r.0 as f64, r.1 as f64, r.2 as f64, r.3 as f64)
}

/// The cells of the unit grid covered by at least one rectangle.
fn cells(rects: &[(i64, i64, i64, i64)], size: i64) -> Vec<Vec<bool>> {
    let n = size as usize + 1;
    let mut grid = vec![vec![false; n]; n];
    for r in rects {
        for x in r.0..r.2 {
            for y in r.1..r.3 {
                if (0..size).contains(&x) && (0..size).contains(&y) {
                    grid[x as usize][y as usize] = true;
                }
            }
        }
    }
    grid
}

#[test]
fn the_union_of_random_rectangles_has_the_area_counted_cell_by_cell() {
    let mut rng = Seeded(0x00C0_FFEE);
    for round in 0..120 {
        let count = 2 + rng.below(7) as usize;
        let rects: Vec<_> = (0..count).map(|_| random_rect(&mut rng, 24)).collect();
        let regions: Vec<Region> = rects.iter().map(|r| rect_region(*r)).collect();
        let result = union_all(&regions, &opts()).unwrap();
        let grid = cells(&rects, 60);
        let oracle = grid.iter().flatten().filter(|c| **c).count() as f64;
        assert!(
            (result.area() - oracle).abs() < 1e-6,
            "round {round}: {} against {oracle} for {rects:?}",
            result.area()
        );
        // every cell centre is inside exactly when the oracle says so
        for (x, column) in grid.iter().enumerate().take(40) {
            for (y, cell) in column.iter().enumerate().take(40) {
                let inside =
                    result.contains((x as f64 + 0.5, y as f64 + 0.5)) == Containment::Inside;
                assert_eq!(inside, *cell, "round {round}: cell {x},{y}");
            }
        }
    }
}

#[test]
fn the_difference_and_the_intersection_of_random_unions_have_the_area_counted_cell_by_cell() {
    let mut rng = Seeded(7);
    for round in 0..80 {
        let ra: Vec<_> = (0..1 + rng.below(5))
            .map(|_| random_rect(&mut rng, 20))
            .collect();
        let rb: Vec<_> = (0..1 + rng.below(5))
            .map(|_| random_rect(&mut rng, 20))
            .collect();
        let a = union_all(
            &ra.iter().map(|r| rect_region(*r)).collect::<Vec<_>>(),
            &opts(),
        )
        .unwrap();
        let b = union_all(
            &rb.iter().map(|r| rect_region(*r)).collect::<Vec<_>>(),
            &opts(),
        )
        .unwrap();
        let (ga, gb) = (cells(&ra, 40), cells(&rb, 40));
        let count = |f: &dyn Fn(bool, bool) -> bool| {
            let mut n = 0.0;
            for (ca, cb) in ga.iter().zip(&gb).take(40) {
                for (p, q) in ca.iter().zip(cb).take(40) {
                    if f(*p, *q) {
                        n += 1.0;
                    }
                }
            }
            n
        };
        let d = combine(Operation::Difference, &a, &b, &opts()).unwrap();
        assert_eq!(d.dropped_edges, 0, "round {round}");
        assert!(
            (d.region.area() - count(&|p, q| p && !q)).abs() < 1e-6,
            "round {round} difference"
        );
        assert!(
            (intersection(&a, &b, &opts()).unwrap().area() - count(&|p, q| p && q)).abs() < 1e-6,
            "round {round} intersection"
        );
        assert!(
            (xor(&a, &b, &opts()).unwrap().area() - count(&|p, q| p != q)).abs() < 1e-6,
            "round {round} xor"
        );
        assert!(
            (union(&a, &b, &opts()).unwrap().area() - count(&|p, q| p || q)).abs() < 1e-6,
            "round {round} union"
        );
    }
}

fn random_polygon(rng: &mut Seeded) -> Region {
    let sides = 3 + rng.below(14) as usize;
    let radius = 1.0 + rng.unit() * 6.0;
    let center = (rng.unit() * 10.0, rng.unit() * 10.0);
    let mut region = Region::regular_polygon(center, radius, sides);
    // turn it
    let turn = rng.unit() * std::f64::consts::TAU;
    for p in &mut region.rings[0] {
        let (dx, dy) = (p.0 - center.0, p.1 - center.1);
        *p = (
            center.0 + dx * turn.cos() - dy * turn.sin(),
            center.1 + dx * turn.sin() + dy * turn.cos(),
        );
    }
    region
}

#[test]
fn the_four_operations_agree_with_each_other_on_random_polygons() {
    let mut rng = Seeded(2024);
    for round in 0..150 {
        let (a, b) = (random_polygon(&mut rng), random_polygon(&mut rng));
        let (aa, ab) = (a.area(), b.area());
        let u = combine(Operation::Union, &a, &b, &opts()).unwrap();
        let i = combine(Operation::Intersection, &a, &b, &opts()).unwrap();
        let d = combine(Operation::Difference, &a, &b, &opts()).unwrap();
        let x = combine(Operation::Xor, &a, &b, &opts()).unwrap();
        for c in [&u, &i, &d, &x] {
            assert_eq!(c.dropped_edges, 0, "round {round}");
        }
        // the grid moves a point by up to half a millionth: the identities hold to the perimeter times that
        let eps = 1e-4;
        assert!(
            (u.region.area() + i.region.area() - aa - ab).abs() < eps,
            "round {round}: inclusion and exclusion"
        );
        assert!(
            (d.region.area() - (aa - i.region.area())).abs() < eps,
            "round {round}: difference"
        );
        assert!(
            (x.region.area() - (u.region.area() - i.region.area())).abs() < eps,
            "round {round}: xor"
        );
        // the order of the operands: the union and the intersection do not care
        let u2 = union(&b, &a, &opts()).unwrap();
        let i2 = intersection(&b, &a, &opts()).unwrap();
        assert!((u2.area() - u.region.area()).abs() < eps);
        assert!((i2.area() - i.region.area()).abs() < eps);
        // the intersection is in the union and a union is never smaller than its parts
        assert!(u.region.area() + eps >= aa.max(ab));
        // the same operand twice
        assert!((union(&a, &a, &opts()).unwrap().area() - aa).abs() < eps);
        assert!(difference(&a, &a, &opts()).unwrap().area() < eps);
        // no ring of a result crosses another
        assert!(no_crossings(&u.region), "round {round}: union rings cross");
        assert!(
            no_crossings(&d.region),
            "round {round}: difference rings cross"
        );
    }
}

fn no_crossings(region: &Region) -> bool {
    let edges: Vec<(Pt, Pt)> = region
        .rings
        .iter()
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .collect();
    let orient = |a: Pt, b: Pt, c: Pt| (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let (p, q) = edges[i];
            let (r, s) = edges[j];
            let eps = 1e-9;
            let (d1, d2, d3, d4) = (
                orient(p, q, r),
                orient(p, q, s),
                orient(r, s, p),
                orient(r, s, q),
            );
            if d1 * d2 < -eps && d3 * d4 < -eps {
                return false;
            }
        }
    }
    true
}

#[test]
fn a_ring_is_never_wound_the_wrong_way_in_a_result() {
    let mut rng = Seeded(99);
    for _ in 0..60 {
        let (a, b) = (random_polygon(&mut rng), random_polygon(&mut rng));
        let big = Region::rectangle(-20.0, -20.0, 40.0, 40.0);
        // a hole made by the difference is clockwise, the outer ring counterclockwise
        let ring = difference(&big, &a, &opts()).unwrap();
        let mut outer = 0;
        let mut hole = 0;
        for r in &ring.rings {
            if signed_area(r) > 0.0 {
                outer += 1;
            } else {
                hole += 1;
            }
        }
        assert_eq!((outer, hole), (1, 1));
        let _ = b;
    }
}

#[test]
fn growing_contains_the_original_and_growing_more_gives_more() {
    let mut rng = Seeded(31337);
    let o = OffsetOptions::default();
    for round in 0..40 {
        let rect = random_rect(&mut rng, 20);
        let r = rect_region(rect);
        let d1 = 0.2 + rng.unit();
        let d2 = d1 + 0.3 + rng.unit();
        let g1 = offset(&r, d1, &o).unwrap();
        let g2 = offset(&r, d2, &o).unwrap();
        assert!(
            g1.area() > r.area() && g2.area() > g1.area(),
            "round {round}"
        );
        // the original is inside the grown shape, and the grown shape in the bigger one
        assert!(
            (difference(&r, &g1, &opts()).unwrap().area()) < 1e-6,
            "round {round}"
        );
        assert!(
            (difference(&g1, &g2, &opts()).unwrap().area()) < 1e-4,
            "round {round}"
        );
        // the area is the exact formula for a rectangle, a little less because the circle is a polygon
        let (w, h) = ((rect.2 - rect.0) as f64, (rect.3 - rect.1) as f64);
        let exact = w * h + 2.0 * d1 * (w + h) + std::f64::consts::PI * d1 * d1;
        assert!(
            g1.area() <= exact + 1e-6 && g1.area() > exact - 0.01 * exact,
            "round {round}: {} vs {exact}",
            g1.area()
        );
        // shrinking and growing again never gives back more than there was
        let eroded = offset(&r, -(d1.min(0.45)), &o).unwrap();
        let back = offset(&eroded, d1.min(0.45), &o).unwrap();
        assert!(
            difference(&back, &r, &opts()).unwrap().area() < 1e-4,
            "round {round}: opening"
        );
    }
}

#[test]
fn a_stroke_covers_its_points_and_nothing_farther_than_half_its_width() {
    let mut rng = Seeded(5);
    let o = OffsetOptions::default();
    for round in 0..40 {
        let n = 2 + rng.below(5) as usize;
        let path: Vec<Pt> = (0..n)
            .map(|_| (rng.unit() * 20.0, rng.unit() * 20.0))
            .collect();
        let width = 0.5 + rng.unit() * 2.0;
        let s = stroke(&path, width, false, &o).unwrap();
        for p in &path {
            assert_eq!(s.contains(*p), Containment::Inside, "round {round}");
        }
        let bounds = s.bounds().unwrap();
        let (minx, maxx) = path
            .iter()
            .fold((f64::MAX, f64::MIN), |m, p| (m.0.min(p.0), m.1.max(p.0)));
        assert!(
            bounds[0] >= minx - width / 2.0 - 1e-6 && bounds[2] <= maxx + width / 2.0 + 1e-6,
            "round {round}"
        );
    }
}

#[test]
fn simplification_leaves_every_point_within_the_tolerance() {
    let mut rng = Seeded(11);
    for round in 0..60 {
        let n = 3 + rng.below(300) as usize;
        let mut pts = vec![(0.0, 0.0)];
        for _ in 1..n {
            let last = *pts.last().unwrap();
            pts.push((
                last.0 + rng.unit() * 2.0 - 0.8,
                last.1 + rng.unit() * 2.0 - 1.0,
            ));
        }
        let tol = 0.05 + rng.unit();
        let out = simplify(&pts, tol, false);
        assert_eq!(out.first(), pts.first());
        assert_eq!(out.last(), pts.last());
        // the points kept are some of the points, in order
        let mut at = 0;
        for p in &out {
            at += pts[at..]
                .iter()
                .position(|q| q == p)
                .expect("a point of the input");
        }
        for p in &pts {
            let d = out
                .windows(2)
                .map(|w| {
                    let (dx, dy) = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                    let len2 = dx * dx + dy * dy;
                    let t = if len2 == 0.0 {
                        0.0
                    } else {
                        (((p.0 - w[0].0) * dx + (p.1 - w[0].1) * dy) / len2).clamp(0.0, 1.0)
                    };
                    (p.0 - w[0].0 - t * dx).hypot(p.1 - w[0].1 - t * dy)
                })
                .fold(f64::INFINITY, f64::min);
            assert!(d <= tol + 1e-9, "round {round}: {d} > {tol}");
        }
    }
}

#[test]
fn a_circle_cut_into_equal_pieces_gives_back_its_radius_by_the_formula() {
    let mut rng = Seeded(808);
    for round in 0..100 {
        let radius = 0.5 + rng.unit() * 50.0;
        // a turn that is not one of the regular figures
        let turn_deg = 2.0 + rng.unit() * 20.0;
        let count = 6 + rng.below(12) as usize;
        let cw = rng.below(2) == 0;
        let center = (rng.unit() * 100.0 - 50.0, rng.unit() * 100.0 - 50.0);
        let start = rng.unit() * 360.0;
        let pts: Vec<Pt> = (0..count)
            .map(|i| {
                let a = (start + (if cw { -1.0 } else { 1.0 }) * turn_deg * i as f64).to_radians();
                (center.0 + radius * a.cos(), center.1 + radius * a.sin())
            })
            .collect();
        let length = ((pts[1].0 - pts[0].0).hypot(pts[1].1 - pts[0].1)).abs();
        let by_formula = radius_from_chain(length, turn_deg.to_radians()).unwrap();
        assert!(
            (by_formula - radius).abs() < 1e-9 * radius.max(1.0),
            "round {round}"
        );
        let pieces = recover_arcs(&pts, &ArcRecovery::default());
        assert_eq!(pieces.len(), 1, "round {round}: {pieces:?}");
        match pieces[0] {
            Piece::Arc {
                center: c,
                radius: r,
                clockwise,
                replaces,
                ..
            } => {
                assert!((r - radius).abs() < 1e-6 * radius.max(1.0), "round {round}");
                assert!(
                    (c.0 - center.0).abs() < 1e-6 && (c.1 - center.1).abs() < 1e-6,
                    "round {round}"
                );
                assert_eq!(clockwise, cw);
                assert_eq!(replaces, count - 1);
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn whatever_the_points_the_pieces_follow_them_in_order_and_nothing_panics() {
    let mut rng = Seeded(4242);
    for _ in 0..200 {
        let n = rng.below(40) as usize;
        let pts: Vec<Pt> = (0..n)
            .map(|_| (rng.unit() * 10.0, rng.unit() * 10.0))
            .collect();
        let pieces = recover_arcs(&pts, &ArcRecovery::default());
        if n < 2 {
            assert!(pieces.is_empty());
            continue;
        }
        let mut at = pts[0];
        for p in &pieces {
            let (from, to) = match p {
                Piece::Line { from, to } | Piece::Arc { from, to, .. } => (*from, *to),
            };
            assert_eq!(from, at);
            at = to;
        }
        assert_eq!(at, pts[n - 1]);
    }
}

#[test]
fn rings_that_cross_themselves_never_panic_and_are_refused_or_cleaned() {
    let mut rng = Seeded(1234);
    for _ in 0..200 {
        // a ring of random points: it usually crosses itself, which is not a valid region
        let n = 3 + rng.below(12) as usize;
        let ring: Vec<Pt> = (0..n)
            .map(|_| (rng.unit() * 10.0, rng.unit() * 10.0))
            .collect();
        let bad = Region::from_ring(ring);
        let good = Region::rectangle(2.0, 2.0, 8.0, 8.0);
        for op in [
            Operation::Union,
            Operation::Intersection,
            Operation::Difference,
            Operation::Xor,
        ] {
            let result = combine(op, &bad, &good, &opts());
            assert!(result.is_ok(), "{result:?}");
        }
        let _ = union(&bad, &good, &opts());
        let _ = offset(&bad, 0.3, &OffsetOptions::default());
    }
}
