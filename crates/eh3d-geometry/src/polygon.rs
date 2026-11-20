//! Polygons and regions: area, centroid, orientation, point in polygon and the simplification of Douglas and
//! Peucker.
//!
//! A **ring** is a closed polygon given as a list of points without the first one repeated at the end. A
//! **region** is a set of rings filled by the **even-odd rule**: a point is inside when a ray from it crosses an odd
//! number of edges of all the rings together. So a ring inside another ring is a hole, a ring inside a hole is an
//! island, and the orientation of a ring does not matter for what it means. The rings of one region must not cross
//! each other or themselves (they may touch); the operations of [`crate::boolean`] and [`crate::offset`] never give a
//! region that does.
//!
//! Everything here is plain `f64` arithmetic and never panics: an empty ring, a ring of fewer than three points or a
//! point that is not finite gives `0.0`, `None` or the input unchanged, as each function says.

/// A point.
pub type Pt = (f64, f64);

/// The way a ring turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Counterclockwise (the signed area is positive).
    CounterClockwise,
    /// Clockwise (the signed area is negative).
    Clockwise,
    /// No area: fewer than three points, or all on a line.
    Degenerate,
}

/// Where a point is with respect to a ring or a region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Containment {
    /// Inside.
    Inside,
    /// Outside.
    Outside,
    /// On an edge or at a corner.
    Boundary,
}

/// The signed area of a ring: positive when it goes counterclockwise, negative when clockwise (shoelace formula,
/// summed from the first point so a ring far from the origin does not lose digits).
pub fn signed_area(ring: &[Pt]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let (ox, oy) = ring[0];
    let mut sum = 0.0;
    for i in 1..ring.len() - 1 {
        let (ax, ay) = (ring[i].0 - ox, ring[i].1 - oy);
        let (bx, by) = (ring[i + 1].0 - ox, ring[i + 1].1 - oy);
        sum += ax * by - bx * ay;
    }
    sum / 2.0
}

/// The area of a ring, always positive or zero.
pub fn area(ring: &[Pt]) -> f64 {
    signed_area(ring).abs()
}

/// The way a ring turns.
pub fn orientation(ring: &[Pt]) -> Orientation {
    let a = signed_area(ring);
    if a > 0.0 {
        Orientation::CounterClockwise
    } else if a < 0.0 {
        Orientation::Clockwise
    } else {
        Orientation::Degenerate
    }
}

/// The centroid (centre of mass) of the area of a ring, or `None` when the ring has no area.
pub fn centroid(ring: &[Pt]) -> Option<Pt> {
    let a = signed_area(ring);
    if a == 0.0 || !a.is_finite() {
        return None;
    }
    let (ox, oy) = ring[0];
    let (mut cx, mut cy) = (0.0, 0.0);
    for i in 1..ring.len() - 1 {
        let (ax, ay) = (ring[i].0 - ox, ring[i].1 - oy);
        let (bx, by) = (ring[i + 1].0 - ox, ring[i + 1].1 - oy);
        let cross = ax * by - bx * ay;
        cx += (ax + bx) * cross;
        cy += (ay + by) * cross;
    }
    let k = 1.0 / (6.0 * a);
    let c = (ox + cx * k, oy + cy * k);
    (c.0.is_finite() && c.1.is_finite()).then_some(c)
}

/// The box `[min x, min y, max x, max y]` of some points, or `None` when there are none.
pub fn bounds(points: &[Pt]) -> Option<[f64; 4]> {
    let first = points.first()?;
    let mut b = [first.0, first.1, first.0, first.1];
    for p in points {
        b[0] = b[0].min(p.0);
        b[1] = b[1].min(p.1);
        b[2] = b[2].max(p.0);
        b[3] = b[3].max(p.1);
    }
    Some(b)
}

fn on_segment(a: Pt, b: Pt, p: Pt) -> bool {
    let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
    let scale = (b.0 - a.0).abs() + (b.1 - a.1).abs() + 1.0;
    cross.abs() <= 1e-12 * scale * scale
        && p.0 >= a.0.min(b.0) - 1e-12 * scale
        && p.0 <= a.0.max(b.0) + 1e-12 * scale
        && p.1 >= a.1.min(b.1) - 1e-12 * scale
        && p.1 <= a.1.max(b.1) + 1e-12 * scale
}

/// Where a point is with respect to a ring.
pub fn ring_contains(ring: &[Pt], p: Pt) -> Containment {
    region_contains(&[ring.to_vec()], p)
}

/// Where a point is with respect to a region (the even-odd rule over all its rings).
pub fn region_contains(rings: &[Vec<Pt>], p: Pt) -> Containment {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if on_segment(a, b, p) {
                return Containment::Boundary;
            }
            if (a.1 > p.1) != (b.1 > p.1) {
                let x = a.0 + (p.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
                if p.0 < x {
                    inside = !inside;
                }
            }
        }
    }
    if inside {
        Containment::Inside
    } else {
        Containment::Outside
    }
}

/// A set of rings filled by the even-odd rule. See the module documentation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Region {
    /// The rings.
    pub rings: Vec<Vec<Pt>>,
}

impl Region {
    /// An empty region.
    pub fn new() -> Region {
        Region::default()
    }

    /// A region of one ring.
    pub fn from_ring(ring: Vec<Pt>) -> Region {
        Region { rings: vec![ring] }
    }

    /// A rectangle.
    pub fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Region {
        Region::from_ring(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    /// A regular polygon of `sides` sides inscribed in a circle (the first corner on the X axis), or the
    /// 3 sided one when fewer are asked.
    pub fn regular_polygon(center: Pt, radius: f64, sides: usize) -> Region {
        let n = sides.max(3);
        Region::from_ring(
            (0..n)
                .map(|i| {
                    let a = std::f64::consts::TAU * i as f64 / n as f64;
                    (center.0 + radius * a.cos(), center.1 + radius * a.sin())
                })
                .collect(),
        )
    }

    /// True when the region has no ring with an area.
    pub fn is_empty(&self) -> bool {
        self.rings.iter().all(|r| r.len() < 3)
    }

    /// The area filled, which is the area of the rings at an even depth minus the ones at an odd depth. It is
    /// exact for rings that do not cross.
    pub fn area(&self) -> f64 {
        let mut total = 0.0;
        for (i, ring) in self.rings.iter().enumerate() {
            // a point of the ring that touches no other ring: rings that touch at a corner are common in results
            let n = ring.len();
            let candidates = (0..n)
                .map(|k| {
                    let (a, b) = (ring[k], ring[(k + 1) % n]);
                    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
                })
                .chain(ring.iter().copied());
            let mut depth = 0;
            for p in candidates {
                let mut clear = true;
                let mut inside = 0;
                for (j, other) in self.rings.iter().enumerate() {
                    if j == i {
                        continue;
                    }
                    match ring_contains(other, p) {
                        Containment::Boundary => {
                            clear = false;
                            break;
                        }
                        Containment::Inside => inside += 1,
                        Containment::Outside => {}
                    }
                }
                if clear {
                    depth = inside;
                    break;
                }
            }
            let a = area(ring);
            if depth % 2 == 0 {
                total += a;
            } else {
                total -= a;
            }
        }
        total
    }

    /// The box `[min x, min y, max x, max y]` of the rings, or `None` for an empty region.
    pub fn bounds(&self) -> Option<[f64; 4]> {
        let all: Vec<Pt> = self.rings.iter().flatten().copied().collect();
        bounds(&all)
    }

    /// Where a point is with respect to the region.
    pub fn contains(&self, p: Pt) -> Containment {
        region_contains(&self.rings, p)
    }

    /// The region with every ring simplified by [`simplify`] (a ring that would have fewer than three points is
    /// kept as it is).
    pub fn simplified(&self, tolerance: f64) -> Region {
        Region {
            rings: self
                .rings
                .iter()
                .map(|r| simplify(r, tolerance, true))
                .collect(),
        }
    }

    /// The region moved.
    pub fn translated(&self, dx: f64, dy: f64) -> Region {
        Region {
            rings: self
                .rings
                .iter()
                .map(|r| r.iter().map(|p| (p.0 + dx, p.1 + dy)).collect())
                .collect(),
        }
    }
}

fn distance_to_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return (p.0 - a.0).hypot(p.1 - a.1);
    }
    let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
    (p.0 - (a.0 + t * dx)).hypot(p.1 - (a.1 + t * dy))
}

/// The simplification of Douglas and Peucker: the fewest points of the line such that every point that was left
/// out is at most `tolerance` from the simplified line. The first and the last point of an open line are always
/// kept. For a `closed` ring the farthest pair of points are the anchors and a ring that would be left with fewer
/// than three points is returned as it was. A tolerance that is negative or not finite changes nothing. It uses a
/// stack, not recursion, so a line of any length is safe.
pub fn simplify(points: &[Pt], tolerance: f64, closed: bool) -> Vec<Pt> {
    let n = points.len();
    if n < 3 || tolerance.is_nan() || tolerance < 0.0 || !tolerance.is_finite() {
        return points.to_vec();
    }
    if points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return points.to_vec();
    }
    // an open line is one piece; a closed ring is cut into two open lines between the first point and the
    // point farthest from it
    let mut keep = vec![false; n];
    keep[0] = true;
    let halves: Vec<Vec<usize>> = if closed {
        let far = (1..n)
            .max_by(|&i, &j| {
                let di = (points[i].0 - points[0].0).hypot(points[i].1 - points[0].1);
                let dj = (points[j].0 - points[0].0).hypot(points[j].1 - points[0].1);
                di.partial_cmp(&dj).unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(n - 1);
        keep[far] = true;
        vec![
            (0..=far).collect(),
            (far..n).chain(std::iter::once(0)).collect(),
        ]
    } else {
        keep[n - 1] = true;
        vec![(0..n).collect()]
    };
    for half in &halves {
        let mut work = vec![(0usize, half.len() - 1)];
        while let Some((lo, hi)) = work.pop() {
            if hi <= lo + 1 {
                continue;
            }
            let (a, b) = (points[half[lo]], points[half[hi]]);
            let mut worst = (0.0, lo);
            for (k, idx) in half.iter().enumerate().take(hi).skip(lo + 1) {
                let d = distance_to_segment(points[*idx], a, b);
                if d > worst.0 {
                    worst = (d, k);
                }
            }
            if worst.0 > tolerance {
                keep[half[worst.1]] = true;
                work.push((lo, worst.1));
                work.push((worst.1, hi));
            }
        }
    }
    let out: Vec<Pt> = points
        .iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, p)| *p)
        .collect();
    if closed && out.len() < 3 {
        points.to_vec()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SQUARE: [Pt; 4] = [(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)];

    #[test]
    fn area_orientation_and_centroid_of_a_square_in_both_directions() {
        assert_eq!(signed_area(&SQUARE), 16.0);
        let reversed: Vec<Pt> = SQUARE.iter().rev().copied().collect();
        assert_eq!(signed_area(&reversed), -16.0);
        assert_eq!(area(&reversed), 16.0);
        assert_eq!(orientation(&SQUARE), Orientation::CounterClockwise);
        assert_eq!(orientation(&reversed), Orientation::Clockwise);
        assert_eq!(centroid(&SQUARE), Some((2.0, 2.0)));
        assert_eq!(centroid(&reversed), Some((2.0, 2.0)));
    }

    #[test]
    fn degenerate_rings_have_no_area_no_centroid_and_do_not_panic() {
        assert_eq!(signed_area(&[]), 0.0);
        assert_eq!(signed_area(&[(1.0, 1.0), (2.0, 2.0)]), 0.0);
        assert_eq!(
            orientation(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]),
            Orientation::Degenerate
        );
        assert_eq!(centroid(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]), None);
        assert_eq!(centroid(&[(f64::NAN, 0.0), (1.0, 0.0), (0.0, 1.0)]), None);
        assert_eq!(bounds(&[]), None);
        assert!(Region::new().is_empty());
    }

    #[test]
    fn the_centroid_far_from_the_origin_keeps_its_digits() {
        let far = 1.0e7;
        let ring: Vec<Pt> = SQUARE.iter().map(|p| (p.0 + far, p.1 + far)).collect();
        let c = centroid(&ring).unwrap();
        assert!((c.0 - (far + 2.0)).abs() < 1e-6 && (c.1 - (far + 2.0)).abs() < 1e-6);
        assert_eq!(signed_area(&ring), 16.0);
    }

    #[test]
    fn a_region_with_a_hole_has_the_area_of_the_ring_less_the_hole() {
        let mut region = Region::from_ring(SQUARE.to_vec());
        region
            .rings
            .push(vec![(1.0, 1.0), (1.0, 3.0), (3.0, 3.0), (3.0, 1.0)]);
        assert_eq!(region.area(), 12.0);
        assert_eq!(region.contains((2.0, 2.0)), Containment::Outside);
        assert_eq!(region.contains((0.5, 0.5)), Containment::Inside);
        assert_eq!(region.contains((1.0, 2.0)), Containment::Boundary);
        assert_eq!(region.contains((9.0, 9.0)), Containment::Outside);
        // an island inside the hole is filled again
        region
            .rings
            .push(vec![(1.5, 1.5), (2.5, 1.5), (2.5, 2.5), (1.5, 2.5)]);
        assert_eq!(region.area(), 13.0);
        assert_eq!(region.bounds(), Some([0.0, 0.0, 4.0, 4.0]));
    }

    #[test]
    fn a_point_on_a_corner_or_an_edge_is_on_the_boundary() {
        assert_eq!(ring_contains(&SQUARE, (0.0, 0.0)), Containment::Boundary);
        assert_eq!(ring_contains(&SQUARE, (2.0, 0.0)), Containment::Boundary);
        assert_eq!(ring_contains(&SQUARE, (4.0, 2.0)), Containment::Boundary);
        assert_eq!(ring_contains(&SQUARE, (4.0001, 2.0)), Containment::Outside);
        assert_eq!(ring_contains(&[], (0.0, 0.0)), Containment::Outside);
    }

    #[test]
    fn douglas_peucker_keeps_the_ends_and_the_corners_and_the_error_is_within_the_tolerance() {
        let line: Vec<Pt> = (0..=100)
            .map(|i| (f64::from(i), if i == 50 { 5.0 } else { 0.0 }))
            .collect();
        let s = simplify(&line, 0.1, false);
        // the two points beside the spike stay: they are far from the chord from the start to the top
        assert_eq!(
            s,
            vec![
                (0.0, 0.0),
                (49.0, 0.0),
                (50.0, 5.0),
                (51.0, 0.0),
                (100.0, 0.0)
            ]
        );
        let flat = simplify(&line, 10.0, false);
        assert_eq!(flat, vec![(0.0, 0.0), (100.0, 0.0)]);
        // every point left out is within the tolerance of the result
        for tol in [0.01, 0.5, 2.0] {
            let wavy: Vec<Pt> = (0..200)
                .map(|i| (f64::from(i) * 0.1, (f64::from(i) * 0.3).sin()))
                .collect();
            let out = simplify(&wavy, tol, false);
            assert!(out.len() < wavy.len());
            for p in &wavy {
                let d = out
                    .windows(2)
                    .map(|w| distance_to_segment(*p, w[0], w[1]))
                    .fold(f64::INFINITY, f64::min);
                assert!(d <= tol + 1e-12, "{d} > {tol}");
            }
        }
    }

    #[test]
    fn a_closed_ring_is_simplified_round_and_never_below_three_points() {
        let mut ring = Vec::new();
        for i in 0..40 {
            ring.push((f64::from(i) * 0.1, 0.0));
        }
        ring.extend([(4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
        let s = simplify(&ring, 0.01, true);
        assert_eq!(s.len(), 4, "{s:?}");
        assert_eq!(area(&s), 16.0);
        let tiny = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 0.001)];
        assert_eq!(simplify(&tiny, 1.0, true), tiny);
        assert_eq!(simplify(&SQUARE, f64::NAN, true), SQUARE.to_vec());
        assert_eq!(simplify(&SQUARE, -1.0, true), SQUARE.to_vec());
        assert_eq!(simplify(&[(0.0, 0.0)], 1.0, false), vec![(0.0, 0.0)]);
        let with_nan = vec![(0.0, 0.0), (f64::NAN, 1.0), (2.0, 0.0)];
        assert_eq!(simplify(&with_nan, 1.0, false).len(), 3);
    }

    #[test]
    fn a_very_long_line_does_not_overflow_the_stack() {
        let straight: Vec<Pt> = (0..200_000).map(|i| (f64::from(i), 0.0)).collect();
        assert_eq!(simplify(&straight, 0.5, false).len(), 2);
        // every point of a zigzag stays: the worst case of the method, kept small
        let zigzag: Vec<Pt> = (0..3000)
            .map(|i| (f64::from(i), if i % 2 == 0 { 0.0 } else { 1.0 }))
            .collect();
        assert_eq!(simplify(&zigzag, 0.1, false).len(), zigzag.len());
    }

    #[test]
    fn a_regular_polygon_and_a_rectangle_have_the_expected_area() {
        let hex = Region::regular_polygon((0.0, 0.0), 1.0, 6);
        assert!((hex.area() - 3.0 * 3f64.sqrt() / 2.0).abs() < 1e-12);
        assert_eq!(Region::rectangle(0.0, 0.0, 2.0, 3.0).area(), 6.0);
        assert_eq!(
            Region::regular_polygon((0.0, 0.0), 1.0, 1).rings[0].len(),
            3
        );
        let moved = Region::rectangle(0.0, 0.0, 2.0, 3.0).translated(5.0, -1.0);
        assert_eq!(moved.bounds(), Some([5.0, -1.0, 7.0, 2.0]));
        assert_eq!(moved.simplified(0.1).area(), 6.0);
    }
}
