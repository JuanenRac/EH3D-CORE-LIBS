//! Offsets of regions and thick lines, with a tolerance.
//!
//! [`offset`] moves every edge of a region outward by a distance (inward when the distance is negative). It is the
//! union (or the difference) of the region with the **band** of points within the distance of its boundary. The band
//! is made of a rectangle on each edge and a wedge of a circle at each corner that turns away from the side being
//! grown, and the union and the difference are the exact ones of [`crate::boolean`], so holes are handled: a hole
//! shrinks when the region grows, and may close.
//!
//! # Corners
//!
//! Growing rounds the corners that stick out: the offset of a square is a square with rounded corners. Shrinking
//! keeps the corners that point into the region sharp, and rounds the notches. This is the offset a tool of that
//! radius makes, and it is the one a CAM needs for isolation (grow the copper by the tool radius) and for clearing
//! (shrink the area by the tool radius).
//!
//! # Tolerance
//!
//! A circle is drawn as a polygon inscribed in it: no point of the result is farther from the true offset than
//! `tolerance`, and the polygon is always on the inside of the true circle. The same tolerance is the grid of the
//! booleans when it is smaller than the grid of the options.

use crate::boolean::{difference, union, union_all, BooleanError, BooleanOptions};
use crate::polygon::{Pt, Region};

/// How an offset is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetOptions {
    /// The most a drawn circle may be inside the true one, in the units of the points. Above zero.
    pub tolerance: f64,
    /// The options of the booleans that join the pieces.
    pub boolean: BooleanOptions,
}

impl Default for OffsetOptions {
    fn default() -> Self {
        OffsetOptions {
            tolerance: 0.001,
            boolean: BooleanOptions::default(),
        }
    }
}

/// Why an offset was refused.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OffsetError {
    /// The tolerance or the distance is not a finite number (the tolerance must be above zero).
    BadNumber,
    /// The distance is so large next to the tolerance that a circle would need more than 100000 sides.
    TooFine,
    /// A boolean of the pieces was refused.
    Boolean(BooleanError),
}

impl From<BooleanError> for OffsetError {
    fn from(e: BooleanError) -> Self {
        OffsetError::Boolean(e)
    }
}

impl std::fmt::Display for OffsetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OffsetError::BadNumber => {
                f.write_str("the distance or the tolerance is not a usable number")
            }
            OffsetError::TooFine => f.write_str("the tolerance is too small for the distance"),
            OffsetError::Boolean(e) => write!(f, "joining the pieces failed: {e}"),
        }
    }
}

impl std::error::Error for OffsetError {}

/// The angle of a side of the polygon that draws a circle of `radius` within `tolerance`.
fn step_angle(radius: f64, tolerance: f64) -> f64 {
    if tolerance >= radius {
        std::f64::consts::FRAC_PI_2
    } else {
        2.0 * (1.0 - tolerance / radius).acos()
    }
}

/// A rectangle round an edge, `distance` to each side.
fn edge_rectangle(p: Pt, q: Pt, distance: f64) -> Option<Region> {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let len = dx.hypot(dy);
    if len == 0.0 || !len.is_finite() {
        return None;
    }
    let (nx, ny) = (-dy / len * distance, dx / len * distance);
    Some(Region::from_ring(vec![
        (p.0 + nx, p.1 + ny),
        (q.0 + nx, q.1 + ny),
        (q.0 - nx, q.1 - ny),
        (p.0 - nx, p.1 - ny),
    ]))
}

/// The wedge of a circle at a corner `v` between the way in `d1` and the way out `d2`, on the side the path turns
/// away from, or `None` when the path does not turn enough to leave a gap.
fn corner_wedge(v: Pt, d1: Pt, d2: Pt, distance: f64, step: f64) -> Option<Region> {
    let a1 = d1.1.atan2(d1.0);
    let a2 = d2.1.atan2(d2.0);
    let mut sweep = a2 - a1;
    while sweep > std::f64::consts::PI {
        sweep -= std::f64::consts::TAU;
    }
    while sweep < -std::f64::consts::PI {
        sweep += std::f64::consts::TAU;
    }
    if sweep.abs() < 1e-9 {
        return None;
    }
    // the normal on the side of the gap: the right of the way in when turning left, the left when turning right
    let start = if sweep > 0.0 {
        a1 - std::f64::consts::FRAC_PI_2
    } else {
        a1 + std::f64::consts::FRAC_PI_2
    };
    let sides = (sweep.abs() / step).ceil().max(1.0) as usize;
    let mut ring = vec![v];
    for i in 0..=sides {
        let a = start + sweep * i as f64 / sides as f64;
        ring.push((v.0 + distance * a.cos(), v.1 + distance * a.sin()));
    }
    Some(Region::from_ring(ring))
}

/// The pieces whose union is the band of points within `distance` of the rings (closed) or of the polyline (open).
fn band_pieces(paths: &[(&[Pt], bool)], distance: f64, step: f64) -> Vec<Region> {
    let mut pieces = Vec::new();
    for (points, closed) in paths {
        let mut pts: Vec<Pt> = Vec::with_capacity(points.len());
        for p in points.iter() {
            if pts.last() != Some(p) {
                pts.push(*p);
            }
        }
        if *closed {
            while pts.len() > 1 && pts.first() == pts.last() {
                pts.pop();
            }
        }
        let n = pts.len();
        if n < 2 {
            continue;
        }
        let edges = if *closed { n } else { n - 1 };
        for i in 0..edges {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            if let Some(r) = edge_rectangle(p, q, distance) {
                pieces.push(r);
            }
        }
        let corners = if *closed {
            0..n
        } else {
            1..n.saturating_sub(1)
        };
        for i in corners {
            let (a, v, b) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let d1 = (v.0 - a.0, v.1 - a.1);
            let d2 = (b.0 - v.0, b.1 - v.1);
            if let Some(w) = corner_wedge(v, d1, d2, distance, step) {
                pieces.push(w);
            }
        }
    }
    pieces
}

fn check(distance: f64, options: &OffsetOptions) -> Result<(f64, f64), OffsetError> {
    if !distance.is_finite() || !options.tolerance.is_finite() || options.tolerance <= 0.0 {
        return Err(OffsetError::BadNumber);
    }
    let r = distance.abs();
    let step = step_angle(r, options.tolerance);
    if r > 0.0 && std::f64::consts::TAU / step > 100_000.0 {
        return Err(OffsetError::TooFine);
    }
    Ok((r, step))
}

/// A region grown by `distance` (shrunk when it is negative). A distance of zero returns the region cleaned by the
/// booleans (no corner on a line, outer rings counterclockwise, holes clockwise).
pub fn offset(
    region: &Region,
    distance: f64,
    options: &OffsetOptions,
) -> Result<Region, OffsetError> {
    let (r, step) = check(distance, options)?;
    if r == 0.0 || region.is_empty() {
        return Ok(union(region, &Region::new(), &options.boolean)?);
    }
    let paths: Vec<(&[Pt], bool)> = region.rings.iter().map(|r| (r.as_slice(), true)).collect();
    let band = union_all(&band_pieces(&paths, r, step), &options.boolean)?;
    if distance > 0.0 {
        Ok(union(region, &band, &options.boolean)?)
    } else {
        Ok(difference(region, &band, &options.boolean)?)
    }
}

/// The region a line of a total `width` covers when it is drawn along `path` with round ends and round corners (the
/// shape of a track drawn with a round aperture, or of the path of a round tool). A path of one point is a disc, and
/// `closed` joins the last point to the first.
pub fn stroke(
    path: &[Pt],
    width: f64,
    closed: bool,
    options: &OffsetOptions,
) -> Result<Region, OffsetError> {
    let (r, step) = check(width / 2.0, options)?;
    if width < 0.0 || r == 0.0 || path.is_empty() {
        return Ok(Region::new());
    }
    let mut pieces = band_pieces(&[(path, closed)], r, step);
    if !closed {
        // round ends: a disc at each end (a path of one point is only the disc)
        let sides = ((std::f64::consts::TAU / step).ceil() as usize).max(8);
        let disc = |c: Pt| {
            Region::from_ring(
                (0..sides)
                    .map(|i| {
                        let a = std::f64::consts::TAU * i as f64 / sides as f64;
                        (c.0 + r * a.cos(), c.1 + r * a.sin())
                    })
                    .collect(),
            )
        };
        pieces.push(disc(path[0]));
        if let Some(last) = path.last() {
            pieces.push(disc(*last));
        }
    }
    Ok(union_all(&pieces, &options.boolean)?)
}

/// The band of the points within `width` inside the edge of a region: the region without its shrunk self. It is the
/// ring of copper kept at the border of a pour, or the area a tool must pass over to clear the edge.
pub fn edge_band(
    region: &Region,
    width: f64,
    options: &OffsetOptions,
) -> Result<Region, OffsetError> {
    if width <= 0.0 {
        check(width, options)?;
        return Ok(Region::new());
    }
    let inner = offset(region, -width, options)?;
    Ok(difference(region, &inner, &options.boolean)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon::Containment;

    fn opts() -> OffsetOptions {
        OffsetOptions {
            tolerance: 0.0005,
            ..OffsetOptions::default()
        }
    }

    fn rel(a: f64, b: f64, e: f64) -> bool {
        (a - b).abs() <= e * b.abs().max(1.0)
    }

    #[test]
    fn a_square_grown_is_a_square_with_rounded_corners() {
        let sq = Region::rectangle(0.0, 0.0, 10.0, 10.0);
        let grown = offset(&sq, 1.0, &opts()).unwrap();
        // the exact area: the square, four strips and four quarter discs
        let exact = 100.0 + 4.0 * 10.0 + std::f64::consts::PI;
        assert!(
            rel(grown.area(), exact, 2e-3),
            "{} vs {exact}",
            grown.area()
        );
        // the polygon is inscribed: it never goes past the exact shape
        assert!(grown.area() <= exact + 1e-6);
        assert_eq!(grown.contains((5.0, 10.9)), Containment::Inside);
        assert_eq!(grown.contains((5.0, 11.1)), Containment::Outside);
        // the corner is cut: the point of the sharp square corner is outside
        assert_eq!(grown.contains((10.9, 10.9)), Containment::Outside);
        assert_eq!(grown.rings.len(), 1);
    }

    #[test]
    fn a_square_shrunk_keeps_its_corners_sharp() {
        let sq = Region::rectangle(0.0, 0.0, 10.0, 10.0);
        let shrunk = offset(&sq, -1.0, &opts()).unwrap();
        assert!(rel(shrunk.area(), 64.0, 1e-6), "{}", shrunk.area());
        assert_eq!(shrunk.rings[0].len(), 4);
        assert_eq!(shrunk.bounds().unwrap()[0], 1.0);
        // too much: nothing is left
        assert!(offset(&sq, -5.0, &opts()).unwrap().is_empty());
        assert!(offset(&sq, -6.0, &opts()).unwrap().is_empty());
        // zero distance cleans and keeps
        assert!(rel(offset(&sq, 0.0, &opts()).unwrap().area(), 100.0, 1e-9));
    }

    #[test]
    fn a_hole_shrinks_when_the_region_grows_and_closes() {
        let mut frame = Region::rectangle(0.0, 0.0, 10.0, 10.0);
        frame
            .rings
            .push(vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)]);
        let grown = offset(&frame, 1.0, &opts()).unwrap();
        assert_eq!(grown.rings.len(), 2, "outer ring and a smaller hole");
        let hole = grown
            .rings
            .iter()
            .find(|r| crate::polygon::signed_area(r) < 0.0)
            .unwrap();
        let b = crate::polygon::bounds(hole).unwrap();
        assert!(rel(b[0], 4.0, 1e-6) && rel(b[2], 6.0, 1e-6));
        let closed = offset(&frame, 2.0, &opts()).unwrap();
        assert_eq!(closed.rings.len(), 1, "a hole of 4 closes when grown by 2");
        let eroded = offset(&frame, -1.0, &opts()).unwrap();
        // the frame is 3 wide: it loses 1 on each side, so a ring 1 wide is left between 1 and 2
        // the hole grows by 1 with its corners rounded (a notch is rounded when shrinking)
        let hole = 36.0 - (4.0 - std::f64::consts::PI);
        assert!(rel(eroded.area(), 64.0 - hole, 2e-3), "{}", eroded.area());
        assert_eq!(eroded.contains((1.5, 5.0)), Containment::Inside);
        assert_eq!(eroded.contains((2.0, 5.0)), Containment::Boundary);
        assert_eq!(eroded.contains((5.0, 5.0)), Containment::Outside);
    }

    #[test]
    fn two_shapes_that_grow_into_each_other_become_one() {
        let two = Region {
            rings: vec![
                vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
                vec![(5.0, 0.0), (9.0, 0.0), (9.0, 4.0), (5.0, 4.0)],
            ],
        };
        assert_eq!(offset(&two, 0.3, &opts()).unwrap().rings.len(), 2);
        assert_eq!(offset(&two, 0.6, &opts()).unwrap().rings.len(), 1);
    }

    #[test]
    fn a_concave_corner_is_not_rounded_when_growing() {
        // an L: the inner corner (reflex) stays sharp
        let l = Region::from_ring(vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 4.0),
            (4.0, 4.0),
            (4.0, 10.0),
            (0.0, 10.0),
        ]);
        let g = offset(&l, 1.0, &opts()).unwrap();
        assert_eq!(
            g.contains((4.9, 4.9)),
            Containment::Inside,
            "the inner corner moved to (5, 5)"
        );
        assert_eq!(g.contains((5.1, 5.1)), Containment::Outside);
        assert!(g.rings[0]
            .iter()
            .any(|p| rel(p.0, 5.0, 1e-6) && rel(p.1, 5.0, 1e-6)));
        // rounding of the six outer corners that point out: five of them (the reflex one is sharp)
        assert!(g.rings[0].len() > 6);
    }

    #[test]
    fn a_stroke_is_a_track_with_round_ends_and_corners() {
        let track = stroke(&[(0.0, 0.0), (10.0, 0.0)], 2.0, false, &opts()).unwrap();
        let exact = 20.0 + std::f64::consts::PI;
        assert!(rel(track.area(), exact, 3e-3), "{}", track.area());
        let bend = stroke(
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            2.0,
            false,
            &opts(),
        )
        .unwrap();
        // two bars of 20 that overlap by 1, a quarter disc at the outer corner and a disc at each end
        let exact = 39.0 + std::f64::consts::FRAC_PI_4 + std::f64::consts::PI;
        assert!(rel(bend.area(), exact, 3e-3), "{}", bend.area());
        let dot = stroke(&[(5.0, 5.0)], 2.0, false, &opts()).unwrap();
        assert!(rel(dot.area(), std::f64::consts::PI, 3e-3));
        let ring = stroke(
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            2.0,
            true,
            &opts(),
        )
        .unwrap();
        assert_eq!(ring.rings.len(), 2, "a closed track is a ring with a hole");
        assert!(stroke(&[], 2.0, false, &opts()).unwrap().is_empty());
        assert!(stroke(&[(0.0, 0.0), (1.0, 0.0)], 0.0, false, &opts())
            .unwrap()
            .is_empty());
        assert!(stroke(&[(0.0, 0.0), (1.0, 0.0)], -1.0, false, &opts())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn the_edge_band_is_the_region_less_its_shrunk_self() {
        let sq = Region::rectangle(0.0, 0.0, 10.0, 10.0);
        let band = edge_band(&sq, 1.0, &opts()).unwrap();
        assert!(rel(band.area(), 36.0, 1e-6));
        assert_eq!(band.rings.len(), 2);
        assert!(edge_band(&sq, 0.0, &opts()).unwrap().is_empty());
        assert!(edge_band(&sq, f64::NAN, &opts()).is_err());
    }

    #[test]
    fn the_numbers_are_checked() {
        let sq = Region::rectangle(0.0, 0.0, 1.0, 1.0);
        assert_eq!(offset(&sq, f64::NAN, &opts()), Err(OffsetError::BadNumber));
        assert_eq!(
            offset(&sq, f64::INFINITY, &opts()),
            Err(OffsetError::BadNumber)
        );
        let bad = OffsetOptions {
            tolerance: 0.0,
            ..opts()
        };
        assert_eq!(offset(&sq, 1.0, &bad), Err(OffsetError::BadNumber));
        let fine = OffsetOptions {
            tolerance: 1e-12,
            ..opts()
        };
        assert_eq!(offset(&sq, 1000.0, &fine), Err(OffsetError::TooFine));
        assert!(OffsetError::TooFine.to_string().contains("tolerance"));
        assert!(offset(&Region::new(), 1.0, &opts()).unwrap().is_empty());
    }

    #[test]
    fn a_finer_tolerance_is_nearer_the_true_circle() {
        let sq = Region::rectangle(0.0, 0.0, 1.0, 1.0);
        let exact = 1.0 + 4.0 * 2.0 + std::f64::consts::PI * 4.0;
        let coarse = offset(
            &sq,
            2.0,
            &OffsetOptions {
                tolerance: 0.1,
                ..opts()
            },
        )
        .unwrap();
        let fine = offset(
            &sq,
            2.0,
            &OffsetOptions {
                tolerance: 0.001,
                ..opts()
            },
        )
        .unwrap();
        assert!((exact - fine.area()).abs() < (exact - coarse.area()).abs());
        assert!(fine.area() <= exact && coarse.area() <= exact);
    }
}
