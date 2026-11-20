//! Arc recovery: finding the arcs that a polyline was made from.
//!
//! Many programs write an arc as a chain of short straight pieces. When all the pieces have the same length `L` and
//! every corner turns by the same angle `phi`, the corners are on a circle of radius
//!
//! ```text
//! R = L / (2 * sin(phi / 2))
//! ```
//!
//! because the chord of a circle of radius `R` that spans the angle `phi` is `2 R sin(phi / 2)` long. [`radius_from_chain`]
//! is that formula. [`recover_arcs`] looks for such chains in a polyline and gives them back as arcs, keeping the
//! rest as straight lines, so a machine can cut one arc where the file had a hundred pieces.
//!
//! # What counts as a chain
//!
//! * at least [`ArcRecovery::min_segments`] pieces (4 by default: three equal turns);
//! * all the pieces of the same length within [`ArcRecovery::length_tolerance`] (a fraction of the length);
//! * all the corners turn the same way, by the same angle within [`ArcRecovery::angle_tolerance_degrees`];
//! * the turn is not larger than [`ArcRecovery::max_turn_degrees`] and is not one of the turns of the common
//!   regular figures ([`ArcRecovery::excluded_turns_degrees`]: 45, 60, 90 and 120 degrees, the octagon, the
//!   hexagon, the square and the triangle), because a chain of those is a polygon the author drew on purpose;
//! * every corner is at most [`ArcRecovery::radius_tolerance`] (a fraction of the radius) from the circle, and the
//!   radius is within [`ArcRecovery::min_radius`] and [`ArcRecovery::max_radius`].
//!
//! The values live in an `ArcRecovery` so a program can read them from its configuration file.

use crate::polygon::Pt;

/// The radius of the circle through the corners of a chain of pieces of length `length` whose corners turn
/// `turn_radians`: `R = L / (2 sin(turn / 2))`. Returns `None` when the turn is zero or not a number between 0 and
/// pi (pieces on a line, or a chain that doubles back).
pub fn radius_from_chain(length: f64, turn_radians: f64) -> Option<f64> {
    let t = turn_radians.abs();
    if !length.is_finite() || length <= 0.0 || !(t > 0.0 && t < std::f64::consts::PI) {
        return None;
    }
    Some(length / (2.0 * (t / 2.0).sin()))
}

/// The settings of the search for arcs.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcRecovery {
    /// The fewest pieces of a chain.
    pub min_segments: usize,
    /// How much the length of a piece may differ from the first one, as a fraction of it.
    pub length_tolerance: f64,
    /// How much a turn may differ from the first one, in degrees.
    pub angle_tolerance_degrees: f64,
    /// The largest turn at a corner, in degrees.
    pub max_turn_degrees: f64,
    /// The turns of regular figures that are never taken for an arc, in degrees; a turn within the angle tolerance
    /// of one of them excludes the chain.
    pub excluded_turns_degrees: Vec<f64>,
    /// How far a corner may be from the circle, as a fraction of the radius.
    pub radius_tolerance: f64,
    /// The smallest radius.
    pub min_radius: f64,
    /// The largest radius.
    pub max_radius: f64,
}

impl Default for ArcRecovery {
    fn default() -> Self {
        ArcRecovery {
            min_segments: 4,
            length_tolerance: 0.02,
            angle_tolerance_degrees: 0.5,
            max_turn_degrees: 30.0,
            excluded_turns_degrees: vec![45.0, 60.0, 90.0, 120.0],
            radius_tolerance: 0.005,
            min_radius: 0.0,
            max_radius: 1.0e6,
        }
    }
}

/// A piece of the recovered path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Piece {
    /// A straight line.
    Line {
        /// Start.
        from: Pt,
        /// End.
        to: Pt,
    },
    /// An arc.
    Arc {
        /// Start.
        from: Pt,
        /// End.
        to: Pt,
        /// Centre.
        center: Pt,
        /// Radius.
        radius: f64,
        /// True when the arc goes clockwise.
        clockwise: bool,
        /// The angle it spans in radians (positive).
        sweep: f64,
        /// How many straight pieces it replaces.
        replaces: usize,
    },
}

fn circumcenter(a: Pt, b: Pt, c: Pt) -> Option<Pt> {
    let d = 2.0 * (a.0 * (b.1 - c.1) + b.0 * (c.1 - a.1) + c.0 * (a.1 - b.1));
    if d == 0.0 || !d.is_finite() {
        return None;
    }
    let (a2, b2, c2) = (
        a.0 * a.0 + a.1 * a.1,
        b.0 * b.0 + b.1 * b.1,
        c.0 * c.0 + c.1 * c.1,
    );
    let ux = (a2 * (b.1 - c.1) + b2 * (c.1 - a.1) + c2 * (a.1 - b.1)) / d;
    let uy = (a2 * (c.0 - b.0) + b2 * (a.0 - c.0) + c2 * (b.0 - a.0)) / d;
    (ux.is_finite() && uy.is_finite()).then_some((ux, uy))
}

fn turn(p: Pt, q: Pt, r: Pt) -> f64 {
    let (ax, ay) = (q.0 - p.0, q.1 - p.1);
    let (bx, by) = (r.0 - q.0, r.1 - q.1);
    (ax * by - ay * bx).atan2(ax * bx + ay * by)
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The path through `points` with the chains that are arcs replaced by arcs. The pieces follow the points in
/// order and the end of one is the start of the next. Fewer than two points give no piece.
pub fn recover_arcs(points: &[Pt], settings: &ArcRecovery) -> Vec<Piece> {
    let n = points.len();
    let mut out = Vec::new();
    if n < 2 || points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
        return out;
    }
    let min_segments = settings.min_segments.max(3);
    let angle_tol = settings.angle_tolerance_degrees.to_radians().abs();
    let max_turn = settings.max_turn_degrees.to_radians().abs();
    let mut i = 0usize;
    while i + 1 < n {
        // the longest chain that starts at point i
        let chain_end = if i + min_segments < n {
            chain(points, i, settings, angle_tol, max_turn)
        } else {
            None
        };
        match chain_end {
            Some((end, center, radius, clockwise)) if end - i >= min_segments => {
                let start_angle = (points[i].1 - center.1).atan2(points[i].0 - center.0);
                let end_angle = (points[end].1 - center.1).atan2(points[end].0 - center.0);
                let mut sweep = if clockwise {
                    start_angle - end_angle
                } else {
                    end_angle - start_angle
                };
                // the sweep of the chain: the sum of its turns, which is below a full circle
                sweep = sweep.rem_euclid(std::f64::consts::TAU);
                out.push(Piece::Arc {
                    from: points[i],
                    to: points[end],
                    center,
                    radius,
                    clockwise,
                    sweep,
                    replaces: end - i,
                });
                i = end;
            }
            _ => {
                out.push(Piece::Line {
                    from: points[i],
                    to: points[i + 1],
                });
                i += 1;
            }
        }
    }
    out
}

/// The end index, the centre, the radius and the direction of the longest chain from `start`, or `None`.
fn chain(
    points: &[Pt],
    start: usize,
    s: &ArcRecovery,
    angle_tol: f64,
    max_turn: f64,
) -> Option<(usize, Pt, f64, bool)> {
    let n = points.len();
    let len0 = dist(points[start], points[start + 1]);
    if len0 <= 0.0 {
        return None;
    }
    let phi0 = turn(points[start], points[start + 1], points[start + 2]);
    let magnitude = phi0.abs();
    if magnitude == 0.0 || magnitude > max_turn {
        return None;
    }
    let degrees = magnitude.to_degrees();
    if s.excluded_turns_degrees
        .iter()
        .any(|e| (degrees - e).abs() <= angle_tol.to_degrees())
    {
        return None;
    }
    let radius = radius_from_chain(len0, magnitude)?;
    if radius < s.min_radius || radius > s.max_radius {
        return None;
    }
    // the centre from the first three corners; the others must be on that circle
    let center = circumcenter(points[start], points[start + 1], points[start + 2])?;
    let mut end = start + 2;
    while end + 1 < n {
        let next = end + 1;
        let len = dist(points[end], points[next]);
        let phi = turn(points[end - 1], points[end], points[next]);
        let on_circle = (dist(points[next], center) - radius).abs() <= s.radius_tolerance * radius;
        if (len - len0).abs() <= s.length_tolerance * len0
            && phi.signum() == phi0.signum()
            && (phi - phi0).abs() <= angle_tol
            && on_circle
        {
            end = next;
        } else {
            break;
        }
    }
    let first_ok = (dist(points[start], center) - radius).abs() <= s.radius_tolerance * radius
        && (dist(points[start + 1], center) - radius).abs() <= s.radius_tolerance * radius
        && (dist(points[start + 2], center) - radius).abs() <= s.radius_tolerance * radius;
    first_ok.then_some((end, center, radius, phi0 < 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle_points(
        center: Pt,
        r: f64,
        from_deg: f64,
        step_deg: f64,
        count: usize,
        cw: bool,
    ) -> Vec<Pt> {
        (0..count)
            .map(|i| {
                let sign = if cw { -1.0 } else { 1.0 };
                let a = (from_deg + sign * step_deg * i as f64).to_radians();
                (center.0 + r * a.cos(), center.1 + r * a.sin())
            })
            .collect()
    }

    #[test]
    fn the_radius_of_a_chain_is_the_length_over_twice_the_sine_of_half_the_turn() {
        // a circle of radius 10 drawn with pieces that turn 10 degrees
        let pts = circle_points((0.0, 0.0), 10.0, 0.0, 10.0, 4, false);
        let l = dist(pts[0], pts[1]);
        let r = radius_from_chain(l, 10f64.to_radians()).unwrap();
        assert!((r - 10.0).abs() < 1e-12, "{r}");
        // a turn of 180 degrees: the chord is the diameter... and a turn of 0 or pi has no circle
        assert_eq!(radius_from_chain(1.0, 0.0), None);
        assert_eq!(radius_from_chain(1.0, std::f64::consts::PI), None);
        assert_eq!(radius_from_chain(0.0, 0.5), None);
        assert_eq!(radius_from_chain(f64::NAN, 0.5), None);
        assert!(
            (radius_from_chain(2.0, std::f64::consts::FRAC_PI_3 * 2.0).unwrap()
                - 2.0 / (2.0 * 3f64.sqrt() / 2.0))
                .abs()
                < 1e-12
        );
        // the sign of the turn does not matter
        assert_eq!(radius_from_chain(1.0, -0.3), radius_from_chain(1.0, 0.3));
    }

    #[test]
    fn a_quarter_circle_of_pieces_becomes_one_arc_with_its_centre_and_radius() {
        let pts = circle_points((5.0, -3.0), 10.0, 0.0, 5.0, 19, false); // 0 to 90 degrees
        let pieces = recover_arcs(&pts, &ArcRecovery::default());
        assert_eq!(pieces.len(), 1, "{pieces:?}");
        match pieces[0] {
            Piece::Arc {
                from,
                to,
                center,
                radius,
                clockwise,
                sweep,
                replaces,
            } => {
                assert_eq!(from, pts[0]);
                assert_eq!(to, pts[18]);
                assert!(
                    (center.0 - 5.0).abs() < 1e-9 && (center.1 + 3.0).abs() < 1e-9,
                    "{center:?}"
                );
                assert!((radius - 10.0).abs() < 1e-9);
                assert!(!clockwise);
                assert!((sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
                assert_eq!(replaces, 18);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_clockwise_chain_is_a_clockwise_arc() {
        let pts = circle_points((0.0, 0.0), 4.0, 90.0, 6.0, 12, true);
        let pieces = recover_arcs(&pts, &ArcRecovery::default());
        assert_eq!(pieces.len(), 1);
        assert!(matches!(
            pieces[0],
            Piece::Arc {
                clockwise: true,
                ..
            }
        ));
    }

    #[test]
    fn lines_on_each_side_of_an_arc_stay_lines() {
        let mut pts = vec![(-10.0, 0.0), (-5.0, 0.0)];
        // the arc starts at (0, 0) going up and to the right
        let arc = circle_points((0.0, 10.0), 10.0, -90.0, 5.0, 10, false);
        let shift = -5.0 - arc[0].0;
        pts.extend(arc.iter().map(|p| (p.0 + shift, p.1)));
        let last = *pts.last().unwrap();
        pts.push((last.0 + 3.0, last.1 + 7.0));
        let pieces = recover_arcs(&pts, &ArcRecovery::default());
        let arcs = pieces
            .iter()
            .filter(|p| matches!(p, Piece::Arc { .. }))
            .count();
        assert_eq!(arcs, 1, "{pieces:?}");
        assert!(matches!(pieces[0], Piece::Line { .. }));
        assert!(matches!(pieces.last().unwrap(), Piece::Line { .. }));
    }

    #[test]
    fn the_regular_figures_are_not_taken_for_arcs() {
        for sides in [3usize, 4, 6, 8] {
            let mut pts =
                circle_points((0.0, 0.0), 5.0, 0.0, 360.0 / sides as f64, sides + 1, false);
            let first = pts[0];
            *pts.last_mut().unwrap() = first;
            let wide = ArcRecovery {
                max_turn_degrees: 130.0,
                ..ArcRecovery::default()
            };
            let pieces = recover_arcs(&pts, &wide);
            assert!(
                pieces.iter().all(|p| matches!(p, Piece::Line { .. })),
                "{sides} sides: {pieces:?}"
            );
        }
        // the same turns are an arc when the program asks for it
        let pts = circle_points((0.0, 0.0), 5.0, 0.0, 45.0, 6, false);
        let loose = ArcRecovery {
            excluded_turns_degrees: Vec::new(),
            max_turn_degrees: 60.0,
            ..ArcRecovery::default()
        };
        assert!(recover_arcs(&pts, &loose)
            .iter()
            .any(|p| matches!(p, Piece::Arc { .. })));
    }

    #[test]
    fn pieces_that_are_not_regular_stay_lines() {
        // equal turns but pieces of growing length: a spiral, not a circle
        let mut pts = vec![(0.0, 0.0)];
        let mut a: f64 = 0.0;
        let mut len = 1.0;
        for _ in 0..12 {
            let last = *pts.last().unwrap();
            pts.push((last.0 + len * a.cos(), last.1 + len * a.sin()));
            a += 8f64.to_radians();
            len *= 1.2;
        }
        assert!(recover_arcs(&pts, &ArcRecovery::default())
            .iter()
            .all(|p| matches!(p, Piece::Line { .. })));
        // a straight line
        let line: Vec<Pt> = (0..10).map(|i| (f64::from(i), 0.0)).collect();
        assert!(recover_arcs(&line, &ArcRecovery::default())
            .iter()
            .all(|p| matches!(p, Piece::Line { .. })));
        // too few pieces
        let few = circle_points((0.0, 0.0), 10.0, 0.0, 5.0, 4, false);
        assert!(recover_arcs(&few, &ArcRecovery::default())
            .iter()
            .all(|p| matches!(p, Piece::Line { .. })));
    }

    #[test]
    fn the_limits_of_radius_and_turn_are_kept() {
        let pts = circle_points((0.0, 0.0), 10.0, 0.0, 5.0, 12, false);
        let small = ArcRecovery {
            max_radius: 9.0,
            ..ArcRecovery::default()
        };
        assert!(recover_arcs(&pts, &small)
            .iter()
            .all(|p| matches!(p, Piece::Line { .. })));
        let big = ArcRecovery {
            min_radius: 11.0,
            ..ArcRecovery::default()
        };
        assert!(recover_arcs(&pts, &big)
            .iter()
            .all(|p| matches!(p, Piece::Line { .. })));
        let sharp = ArcRecovery {
            max_turn_degrees: 4.0,
            ..ArcRecovery::default()
        };
        assert!(recover_arcs(&pts, &sharp)
            .iter()
            .all(|p| matches!(p, Piece::Line { .. })));
    }

    #[test]
    fn nothing_is_made_of_too_few_or_unusable_points() {
        assert!(recover_arcs(&[], &ArcRecovery::default()).is_empty());
        assert!(recover_arcs(&[(0.0, 0.0)], &ArcRecovery::default()).is_empty());
        assert_eq!(
            recover_arcs(&[(0.0, 0.0), (1.0, 0.0)], &ArcRecovery::default()).len(),
            1
        );
        assert!(recover_arcs(
            &[(0.0, 0.0), (f64::NAN, 0.0), (1.0, 1.0)],
            &ArcRecovery::default()
        )
        .is_empty());
        let same = vec![(1.0, 1.0); 10];
        assert!(recover_arcs(&same, &ArcRecovery::default())
            .iter()
            .all(|p| matches!(p, Piece::Line { .. })));
    }

    #[test]
    fn a_recovered_arc_is_within_the_tolerance_of_the_pieces_it_replaces() {
        let pts = circle_points((0.0, 0.0), 25.0, 10.0, 3.0, 40, false);
        let pieces = recover_arcs(&pts, &ArcRecovery::default());
        assert_eq!(pieces.len(), 1);
        if let Piece::Arc { center, radius, .. } = pieces[0] {
            for p in &pts {
                assert!((dist(*p, center) - radius).abs() < 1e-9);
            }
        }
    }
}
