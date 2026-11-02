//! Problem zones: places where the tool can be expected to leave copper behind or to touch it, found before anything is milled. Two kinds are looked for, both with criteria that the
//! configuration file sets: inner corners sharper than a given angle (a round tool cannot reach the tip of a notch in the copper) and tracks that run between two pads that are
//! closer to the track than a multiple of the tool.

use crate::bitmap::from_scene;
use crate::isolate::{close_pieces, label};
use eh3d_geometry::scene::{Scene, Shape};
use eh3d_raster::edt::distance_mm;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProblemRules {
    /// The tool that isolates; it is what has to reach into the corners and between the pads.
    pub tool_mm: f64,
    /// Inner corners (a notch of empty ground between copper) sharper than this angle are reported; 0 does not look for them.
    pub corner_angle_deg: f64,
    /// A track with a pad on each side, each one nearer to it than this multiple of the tool, is reported; 0 does not look for them.
    pub track_gap_factor: f64,
}

impl ProblemRules {
    pub fn validate(&self) -> Result<(), String> {
        if !(self.tool_mm.is_finite() && (0.02..=10.0).contains(&self.tool_mm)) {
            return Err(format!(
                "fine_tool_mm is {}: it must be from 0.02 to 10",
                self.tool_mm
            ));
        }
        if !(self.corner_angle_deg == 0.0
            || (self.corner_angle_deg.is_finite()
                && (10.0..=89.0).contains(&self.corner_angle_deg)))
        {
            return Err(format!(
                "problem_corner_angle_deg is {}: it must be 0 (off) or from 10 to 89",
                self.corner_angle_deg
            ));
        }
        if !(self.track_gap_factor.is_finite() && (0.0..=10.0).contains(&self.track_gap_factor)) {
            return Err(format!(
                "problem_track_gap_factor is {}: it must be from 0 (off) to 10",
                self.track_gap_factor
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProblemKind {
    /// An inner corner sharper than the rule: the round tool leaves copper in the tip.
    SharpCorner,
    /// A track between two pads that are too near it for the tool.
    TrackBetweenPads,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Problem {
    pub kind: ProblemKind,
    pub x: f64,
    pub y: f64,
    /// For a corner, the angle of the rule; for a track, the narrowest gap found (mm).
    pub value: f64,
}

impl Problem {
    pub fn describe(&self) -> String {
        match self.kind {
            ProblemKind::SharpCorner => format!(
                "inner corner sharper than {} degrees, the tool leaves copper there, near ({:.3}, {:.3})",
                self.value, self.x, self.y
            ),
            ProblemKind::TrackBetweenPads => format!(
                "track between two pads with only {:.3} mm to one of them, near ({:.3}, {:.3})",
                self.value, self.x, self.y
            ),
        }
    }
}

/// The area a round tool of radius `r` leaves in the tip of an inner corner of the given angle (degrees): the corner area minus the part of it that the circle covers.
pub fn corner_residue_mm2(r: f64, angle_deg: f64) -> f64 {
    let a = angle_deg.to_radians();
    r * r * (1.0 / (a / 2.0).tan() - (std::f64::consts::PI - a) / 2.0)
}

/// The inner corners that the tool cannot reach, as the places where they are.
fn sharp_corners(scene: &Scene, rules: &ProblemRules, res: f64) -> Result<Vec<Problem>, String> {
    let r = rules.tool_mm / 2.0;
    let bm = from_scene(scene, rules.tool_mm + 4.0 * res, res)?;
    let dist = distance_mm(&bm.on, bm.w, bm.h, res);
    // where the centre of the tool can be, and the ground the tool covers from there
    let centre: Vec<bool> = (0..bm.w * bm.h)
        .map(|i| !bm.on[i] && f64::from(dist[i]) >= r)
        .collect();
    if !centre.iter().any(|c| *c) {
        return Ok(Vec::new());
    }
    let reach = distance_mm(&centre, bm.w, bm.h, res);
    let missed: Vec<bool> = (0..bm.w * bm.h)
        .map(|i| !bm.on[i] && f64::from(reach[i]) > r + res)
        .collect();
    let (ids, count) = label(&missed, bm.w, bm.h);
    let mut sums = vec![(0.0f64, 0.0f64, 0usize); count as usize + 1];
    let mut boxes = vec![(usize::MAX, usize::MAX, 0usize, 0usize); count as usize + 1];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            let (x, y) = (i % bm.w, i / bm.w);
            let s = &mut sums[*id as usize];
            s.0 += x as f64;
            s.1 += y as f64;
            s.2 += 1;
            let b = &mut boxes[*id as usize];
            *b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
    }
    // the gaps narrower than the tool are another finding (they leave a long thin strip, not a corner): leave out what lies on one
    let gaps = close_pieces(scene, rules.tool_mm, res)?;
    let on_gap = |b: &(usize, usize, usize, usize)| {
        let (lo, hi) = (
            bm.centre(b.0 as f64, b.1 as f64),
            bm.centre(b.2 as f64, b.3 as f64),
        );
        gaps.iter().any(|g| {
            g.x >= lo.0 - rules.tool_mm
                && g.x <= hi.0 + rules.tool_mm
                && g.y >= lo.1 - rules.tool_mm
                && g.y <= hi.1 + rules.tool_mm
        })
    };
    let least = corner_residue_mm2(r, rules.corner_angle_deg).max(0.0);
    let mut out = Vec::new();
    for id in 1..=count as usize {
        let s = sums[id];
        if s.2 == 0 || (s.2 as f64) * res * res < least || on_gap(&boxes[id]) {
            continue;
        }
        let (x, y) = bm.centre(s.0 / s.2 as f64, s.1 / s.2 as f64);
        out.push(Problem {
            kind: ProblemKind::SharpCorner,
            x,
            y,
            value: rules.corner_angle_deg,
        });
    }
    Ok(out)
}

/// The distance from a point to the edge of a pad (negative inside it); `None` for a shape that is not a pad.
fn pad_distance(s: &Shape, px: f64, py: f64) -> Option<f64> {
    match s {
        Shape::Disc { x, y, r } => Some((px - x).hypot(py - y) - r),
        Shape::Rect { x, y, w, h } => {
            let (dx, dy) = ((px - x).abs() - w / 2.0, (py - y).abs() - h / 2.0);
            Some(dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0))
        }
        Shape::Obround { x, y, w, h } => {
            // a capsule: the distance to the line segment along the long side, less the short half side
            let (half_long, half_short, horizontal) = if w >= h {
                (w / 2.0 - h / 2.0, h / 2.0, true)
            } else {
                (h / 2.0 - w / 2.0, w / 2.0, false)
            };
            let (u, v) = if horizontal {
                (px - x, py - y)
            } else {
                (py - y, px - x)
            };
            let along = u.abs() - half_long;
            Some(along.max(0.0).hypot(v) - half_short)
        }
        _ => None,
    }
}

fn pad_box(s: &Shape) -> Option<(f64, f64, f64, f64)> {
    match s {
        Shape::Disc { x, y, r } => Some((x - r, y - r, x + r, y + r)),
        Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => {
            Some((x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0))
        }
        _ => None,
    }
}

/// The tracks that run between two pads which are nearer to the track than `limit_mm` (edge to edge) without touching it.
fn tracks_between_pads(scene: &Scene, limit_mm: f64, res: f64) -> Vec<Problem> {
    let pads: Vec<&Shape> = scene
        .shapes
        .iter()
        .filter(|s| pad_box(s).is_some())
        .collect();
    let mut out = Vec::new();
    for t in &scene.shapes {
        let Shape::Stroke { x0, y0, x1, y1, w } = t else {
            continue;
        };
        let reach = limit_mm + w / 2.0;
        let (bx0, by0, bx1, by1) = (
            x0.min(*x1) - reach,
            y0.min(*y1) - reach,
            x0.max(*x1) + reach,
            y0.max(*y1) + reach,
        );
        let length = (x1 - x0).hypot(y1 - y0);
        let steps = (length / res.max(0.01)).ceil().clamp(1.0, 5000.0) as usize;
        // for every pad near enough: its narrowest gap to the track and where it is
        let mut near: Vec<(f64, (f64, f64))> = Vec::new();
        for p in &pads {
            let Some((a, b, c, d)) = pad_box(p) else {
                continue;
            };
            if c < bx0 || a > bx1 || d < by0 || b > by1 {
                continue;
            }
            let mut best = (f64::INFINITY, (0.0, 0.0));
            for k in 0..=steps {
                let f = k as f64 / steps as f64;
                let (px, py) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                if let Some(d) = pad_distance(p, px, py) {
                    let gap = d - w / 2.0;
                    if gap < best.0 {
                        best = (gap, (px, py));
                    }
                }
            }
            // a pad the track touches or crosses is its own; only the ones beside it count
            if best.0 > 1e-6 && best.0 < limit_mm {
                near.push(best);
            }
        }
        if near.len() >= 2 {
            near.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let (gap, (x, y)) = near[0];
            out.push(Problem {
                kind: ProblemKind::TrackBetweenPads,
                x,
                y,
                value: gap,
            });
        }
    }
    out
}

/// The problem zones of a layer, sorted by kind and place.
pub fn problem_zones(
    scene: &Scene,
    rules: &ProblemRules,
    res_mm: f64,
) -> Result<Vec<Problem>, String> {
    rules.validate()?;
    if !(res_mm.is_finite() && (0.002..=1.0).contains(&res_mm)) {
        return Err(format!(
            "the cell is {res_mm} mm: it must be from 0.002 to 1"
        ));
    }
    let mut out = Vec::new();
    if rules.corner_angle_deg > 0.0 {
        if res_mm > rules.tool_mm / 2.0 {
            return Err(format!(
                "the cell of {res_mm} mm is too coarse for a tool of {} mm: it must be at most half of it",
                rules.tool_mm
            ));
        }
        out.extend(sharp_corners(scene, rules, res_mm)?);
    }
    if rules.track_gap_factor > 0.0 {
        out.extend(tracks_between_pads(
            scene,
            rules.track_gap_factor * rules.tool_mm,
            res_mm,
        ));
    }
    out.sort_by(|a, b| {
        (a.kind, a.y, a.x)
            .partial_cmp(&(b.kind, b.y, b.x))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}

/// A track or a rectangular pad that is thin for the tolerance in use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThinCopper {
    pub x: f64,
    pub y: f64,
    /// The width of the track, or the smaller side of the pad.
    pub size_mm: f64,
}

/// The tracks and the rectangular or obround pads narrower than twice the tolerance plus `margin_mm`: with a tolerance of that size two gaps that are accepted on either side
/// of one of them leave almost nothing of it, so the person is told which they are. Nothing is reported without a tolerance. Sorted from the thinnest.
pub fn thin_for_tolerance(
    scene: &Scene,
    tolerance_mm: f64,
    margin_mm: f64,
) -> Result<Vec<ThinCopper>, String> {
    thin_for_tolerance_in(scene, tolerance_mm, margin_mm, false)
}

/// Like `thin_for_tolerance`; with `pads_only` the tracks are left out (the tolerance is meant for pads alone).
pub fn thin_for_tolerance_in(
    scene: &Scene,
    tolerance_mm: f64,
    margin_mm: f64,
    pads_only: bool,
) -> Result<Vec<ThinCopper>, String> {
    if !(tolerance_mm.is_finite() && tolerance_mm >= 0.0) {
        return Err(format!(
            "the tolerance is {tolerance_mm} mm: it must be 0 or more"
        ));
    }
    if !(margin_mm.is_finite() && margin_mm >= 0.0) {
        return Err(format!(
            "the margin is {margin_mm} mm: it must be 0 or more"
        ));
    }
    if tolerance_mm == 0.0 {
        return Ok(Vec::new());
    }
    let limit = 2.0 * tolerance_mm + margin_mm;
    let mut out: Vec<ThinCopper> = scene
        .shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Stroke { x0, y0, x1, y1, w } if !pads_only => Some(ThinCopper {
                x: (x0 + x1) / 2.0,
                y: (y0 + y1) / 2.0,
                size_mm: *w,
            }),
            Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => Some(ThinCopper {
                x: *x,
                y: *y,
                size_mm: w.min(*h),
            }),
            _ => None,
        })
        .filter(|c| c.size_mm < limit)
        .collect();
    out.sort_by(|a, b| {
        (a.size_mm, a.y, a.x)
            .partial_cmp(&(b.size_mm, b.y, b.x))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}
