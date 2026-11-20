//! The geometry of an arc of a Gerber file: its centre, its radius and its sweep, in single and in
//! multi quadrant mode, and its points.
//!
//! The file gives the start, the end and the offsets `I` and `J` from the start to the centre. In multi quadrant
//! mode (`G75`) the offsets are signed and the arc may be a full circle. In single quadrant mode (`G74`,
//! deprecated) the offsets have no sign, the arc never spans more than 90 degrees and the centre is the one of the
//! four possible that fits. Arcs inside a region (`G36`/`G37`) follow the same rules.

use crate::{Point, QuadrantMode};

/// An arc worked out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcGeometry {
    /// The centre.
    pub center: Point,
    /// The radius, taken from the start point.
    pub radius: f64,
    /// The angle of the start point in radians, from -pi to pi.
    pub start_angle: f64,
    /// The signed sweep in radians: positive counterclockwise, negative clockwise. A full circle is `+-2*pi`.
    pub sweep: f64,
    /// How far the distance from the centre to the end point differs from the radius. The files of some
    /// programs round their offsets; a value that is not small means the arc is damaged.
    pub radius_error: f64,
}

const TWO_PI: f64 = std::f64::consts::TAU;

fn angle(c: Point, p: Point) -> f64 {
    (p.y - c.y).atan2(p.x - c.x)
}

/// A sweep of 0 to `2*pi` for the direction asked, from the angle of the start to the angle of the end.
fn sweep_between(a0: f64, a1: f64, clockwise: bool) -> f64 {
    let ccw = (a1 - a0).rem_euclid(TWO_PI);
    if clockwise {
        -((TWO_PI - ccw).rem_euclid(TWO_PI))
    } else {
        ccw
    }
}

/// The geometry of an arc, or `None` when it has no centre to speak of (a zero radius, or numbers that are not
/// finite).
///
/// In multi quadrant mode, a start that equals the end is a full circle. In single quadrant mode it is a
/// point and the sweep is 0. `tolerance` is the distance under which the start and the end are the same point.
pub fn arc_geometry(
    from: Point,
    to: Point,
    offset: (f64, f64),
    clockwise: bool,
    quadrant: QuadrantMode,
) -> Option<ArcGeometry> {
    let all = [from.x, from.y, to.x, to.y, offset.0, offset.1];
    if all.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let scale = all.iter().fold(0.0_f64, |m, v| m.max(v.abs())).max(1.0);
    let same = (from.x - to.x).abs() <= scale * 1e-12 && (from.y - to.y).abs() <= scale * 1e-12;
    match quadrant {
        QuadrantMode::Multi => {
            let center = Point {
                x: from.x + offset.0,
                y: from.y + offset.1,
            };
            let radius = (from.x - center.x).hypot(from.y - center.y);
            if radius <= 0.0 {
                return None;
            }
            let a0 = angle(center, from);
            let sweep = if same {
                if clockwise {
                    -TWO_PI
                } else {
                    TWO_PI
                }
            } else {
                sweep_between(a0, angle(center, to), clockwise)
            };
            Some(ArcGeometry {
                center,
                radius,
                start_angle: a0,
                sweep,
                radius_error: ((to.x - center.x).hypot(to.y - center.y) - radius).abs(),
            })
        }
        QuadrantMode::Single => {
            let (i, j) = (offset.0.abs(), offset.1.abs());
            let mut best: Option<ArcGeometry> = None;
            for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                let center = Point {
                    x: from.x + sx * i,
                    y: from.y + sy * j,
                };
                let radius = (sx * i).hypot(sy * j);
                if radius <= 0.0 {
                    return None;
                }
                let a0 = angle(center, from);
                let sweep = if same {
                    0.0
                } else {
                    sweep_between(a0, angle(center, to), clockwise)
                };
                // an arc of one quadrant spans at most a quarter of a turn (a little more for rounding)
                if sweep.abs() > TWO_PI / 4.0 * (1.0 + 1e-6) {
                    continue;
                }
                let candidate = ArcGeometry {
                    center,
                    radius,
                    start_angle: a0,
                    sweep,
                    radius_error: ((to.x - center.x).hypot(to.y - center.y) - radius).abs(),
                };
                match best {
                    Some(b) if candidate.radius_error >= b.radius_error => {}
                    _ => best = Some(candidate),
                }
            }
            best
        }
    }
}

impl ArcGeometry {
    /// The point of the arc after the fraction `t` of its sweep (0 is the start, 1 the end).
    pub fn point_at(&self, t: f64) -> Point {
        let a = self.start_angle + self.sweep * t;
        Point {
            x: self.center.x + self.radius * a.cos(),
            y: self.center.y + self.radius * a.sin(),
        }
    }

    /// The points of the arc as a polyline whose sides are never farther than `tolerance` from the arc.
    /// The first point is the start and the last the end. At most 100000 sides are made.
    pub fn flatten(&self, tolerance: f64) -> Vec<Point> {
        let tolerance = if tolerance.is_finite() && tolerance > 0.0 {
            tolerance.min(self.radius)
        } else {
            self.radius * 1e-3
        };
        let step = 2.0 * (1.0 - tolerance / self.radius).clamp(-1.0, 1.0).acos();
        let sides = if step > 0.0 {
            (self.sweep.abs() / step).ceil().clamp(1.0, 100_000.0) as usize
        } else {
            100_000
        };
        (0..=sides)
            .map(|i| self.point_at(i as f64 / sides as f64))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn a_quarter_circle_in_both_directions() {
        let ccw = arc_geometry(
            pt(1.0, 0.0),
            pt(0.0, 1.0),
            (-1.0, 0.0),
            false,
            QuadrantMode::Multi,
        )
        .unwrap();
        assert!((ccw.sweep - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert!((ccw.radius - 1.0).abs() < 1e-12 && ccw.radius_error < 1e-12);
        let cw = arc_geometry(
            pt(1.0, 0.0),
            pt(0.0, 1.0),
            (-1.0, 0.0),
            true,
            QuadrantMode::Multi,
        )
        .unwrap();
        assert!((cw.sweep + 3.0 * std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn a_start_that_is_the_end_is_a_full_circle_only_in_multi_quadrant() {
        let m = arc_geometry(
            pt(2.0, 0.0),
            pt(2.0, 0.0),
            (-1.0, 0.0),
            false,
            QuadrantMode::Multi,
        )
        .unwrap();
        assert!((m.sweep - TWO_PI).abs() < 1e-12);
        let m = arc_geometry(
            pt(2.0, 0.0),
            pt(2.0, 0.0),
            (-1.0, 0.0),
            true,
            QuadrantMode::Multi,
        )
        .unwrap();
        assert!((m.sweep + TWO_PI).abs() < 1e-12);
        let s = arc_geometry(
            pt(2.0, 0.0),
            pt(2.0, 0.0),
            (1.0, 0.0),
            false,
            QuadrantMode::Single,
        )
        .unwrap();
        assert_eq!(s.sweep, 0.0);
    }

    #[test]
    fn single_quadrant_finds_the_centre_among_the_four_signs() {
        // a quarter of a circle of radius 1 round (0, 0), for each quadrant and direction
        for (from, to, centre, clockwise) in [
            (pt(1.0, 0.0), pt(0.0, 1.0), pt(0.0, 0.0), false),
            (pt(0.0, 1.0), pt(-1.0, 0.0), pt(0.0, 0.0), false),
            (pt(-1.0, 0.0), pt(0.0, -1.0), pt(0.0, 0.0), false),
            (pt(0.0, -1.0), pt(1.0, 0.0), pt(0.0, 0.0), false),
            (pt(0.0, 1.0), pt(1.0, 0.0), pt(0.0, 0.0), true),
            (pt(1.0, 0.0), pt(0.0, -1.0), pt(0.0, 0.0), true),
        ] {
            // the offsets are given without a sign
            let off = ((centre.x - from.x).abs(), (centre.y - from.y).abs());
            let g = arc_geometry(from, to, off, clockwise, QuadrantMode::Single).unwrap();
            assert!((g.center.x - centre.x).abs() < 1e-12, "{from:?} {to:?}");
            assert!((g.center.y - centre.y).abs() < 1e-12, "{from:?} {to:?}");
            assert!((g.sweep.abs() - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
            assert_eq!(g.sweep < 0.0, clockwise);
        }
    }

    #[test]
    fn nothing_is_made_of_a_zero_radius_or_numbers_that_are_not_finite() {
        assert!(arc_geometry(
            pt(1.0, 1.0),
            pt(2.0, 2.0),
            (0.0, 0.0),
            false,
            QuadrantMode::Multi
        )
        .is_none());
        assert!(arc_geometry(
            pt(1.0, 1.0),
            pt(2.0, 2.0),
            (0.0, 0.0),
            false,
            QuadrantMode::Single
        )
        .is_none());
        assert!(arc_geometry(
            pt(f64::NAN, 1.0),
            pt(2.0, 2.0),
            (1.0, 0.0),
            false,
            QuadrantMode::Multi
        )
        .is_none());
        assert!(arc_geometry(
            pt(1.0, 1.0),
            pt(2.0, 2.0),
            (f64::INFINITY, 0.0),
            true,
            QuadrantMode::Multi
        )
        .is_none());
    }

    #[test]
    fn a_flattened_arc_stays_within_the_tolerance_and_ends_where_the_arc_ends() {
        let g = arc_geometry(
            pt(10.0, 0.0),
            pt(0.0, 10.0),
            (-10.0, 0.0),
            false,
            QuadrantMode::Multi,
        )
        .unwrap();
        let tol = 0.001;
        let pts = g.flatten(tol);
        assert!((pts[0].x - 10.0).abs() < 1e-9 && (pts.last().unwrap().y - 10.0).abs() < 1e-9);
        for pair in pts.windows(2) {
            let mid = pt((pair[0].x + pair[1].x) / 2.0, (pair[0].y + pair[1].y) / 2.0);
            let d = (mid.x.hypot(mid.y) - 10.0).abs();
            assert!(d <= tol * 1.0001, "{d}");
        }
        assert!(g.flatten(f64::NAN).len() > 2 && g.flatten(-1.0).len() > 2);
        assert!(g.flatten(1e300).len() >= 2);
    }

    #[test]
    fn far_from_the_origin_the_arc_keeps_its_precision() {
        // a board 1.5 metres from the origin, in millimetres: the radius error stays far below a micrometre
        let c = 1.5e6;
        let g = arc_geometry(
            pt(c + 5.0, c),
            pt(c, c + 5.0),
            (-5.0, 0.0),
            false,
            QuadrantMode::Multi,
        )
        .unwrap();
        assert!(g.radius_error < 1e-6, "{}", g.radius_error);
        let mid = g.point_at(0.5);
        let r = (mid.x - c).hypot(mid.y - c);
        assert!((r - 5.0).abs() < 1e-6);
    }
}
