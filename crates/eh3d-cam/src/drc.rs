//! A check of the layer against the design rules before anything is milled: copper narrower than the rule, copper pieces closer than the rule or than the tool, and the ring of copper
//! round the holes. Every finding comes with the place it is at, in mm.

use crate::bitmap::from_scene;
use crate::hole::Hole;
use crate::isolate::{close_pieces, label};
use eh3d_geometry::scene::Scene;
use eh3d_raster::edt::distance_mm;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rules {
    /// The narrowest track or neck of copper that is allowed.
    pub min_width_mm: f64,
    /// The least space allowed between two pieces of copper.
    pub min_clearance_mm: f64,
    /// The copper that must be left round a hole, from its edge; 0 does not check it.
    pub annular_ring_mm: f64,
    /// The isolation tool: it cannot go between pieces of copper that are closer than it is wide.
    pub tool_mm: f64,
    /// Report the holes that are not on copper at all (otherwise they are taken to be holes of the board).
    pub holes_on_copper: bool,
}

impl Rules {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("rule_min_width_mm", self.min_width_mm, 0.02, 5.0)?;
        range("rule_min_clearance_mm", self.min_clearance_mm, 0.02, 5.0)?;
        range("rule_annular_ring_mm", self.annular_ring_mm, 0.0, 5.0)?;
        range("fine_tool_mm", self.tool_mm, 0.02, 10.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// Two pieces of copper are closer than the rule.
    Clearance,
    /// Two pieces of copper are closer than the tool is wide.
    ClearanceForTool,
    /// A track or neck of copper is narrower than the rule.
    Width,
    /// A hole has less copper round it than the rule.
    AnnularRing,
    /// A hole is not on copper.
    HoleOffCopper,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Violation {
    pub kind: Kind,
    pub x: f64,
    pub y: f64,
    /// The value of the rule that is not met, in mm.
    pub limit_mm: f64,
}

impl Violation {
    pub fn describe(&self) -> String {
        let what = match self.kind {
            Kind::Clearance => "copper closer than the clearance rule",
            Kind::ClearanceForTool => "copper closer than the tool is wide",
            Kind::Width => "copper narrower than the width rule",
            Kind::AnnularRing => "less copper round the hole than the ring rule",
            Kind::HoleOffCopper => "a hole that is not on copper",
        };
        format!(
            "{what} ({} mm) near ({:.3}, {:.3})",
            self.limit_mm, self.x, self.y
        )
    }
}

/// Copper cells narrower than `width_mm`, as the places where they are: the cells that no circle of that width fits in, gathered in groups. The rounded-off corners that every
/// square corner has are not reported.
fn narrow_copper(scene: &Scene, width_mm: f64, res: f64) -> Result<Vec<(f64, f64)>, String> {
    let bm = from_scene(scene, width_mm + 4.0 * res, res)?;
    let empty: Vec<bool> = bm.on.iter().map(|b| !*b).collect();
    let inner = distance_mm(&empty, bm.w, bm.h, res);
    let r = width_mm / 2.0;
    let thick: Vec<bool> = (0..bm.w * bm.h)
        .map(|i| bm.on[i] && f64::from(inner[i]) >= r)
        .collect();
    let near = distance_mm(&thick, bm.w, bm.h, res);
    let narrow: Vec<bool> = (0..bm.w * bm.h)
        .map(|i| bm.on[i] && f64::from(near[i]) > r + res)
        .collect();
    let (ids, count) = label(&narrow, bm.w, bm.h);
    let mut sums = vec![(0.0f64, 0.0f64, 0usize); count as usize + 1];
    for (i, id) in ids.iter().enumerate() {
        if *id != 0 {
            let s = &mut sums[*id as usize];
            s.0 += (i % bm.w) as f64;
            s.1 += (i / bm.w) as f64;
            s.2 += 1;
        }
    }
    // a square corner loses 0.22 r^2 to the circle; anything smaller than that is no neck
    let least = ((0.3 * r * r) / (res * res)).ceil().max(4.0) as usize;
    Ok(sums
        .iter()
        .skip(1)
        .filter(|s| s.2 >= least)
        .map(|s| bm.centre(s.0 / s.2 as f64, s.1 / s.2 as f64))
        .collect())
}

/// Checks the layer (and the holes, when there are any) against the rules.
pub fn check(
    scene: &Scene,
    holes: &[Hole],
    rules: &Rules,
    res_mm: f64,
) -> Result<Vec<Violation>, String> {
    rules.validate()?;
    if !(res_mm.is_finite() && (0.002..=1.0).contains(&res_mm)) {
        return Err(format!(
            "the cell is {res_mm} mm: it must be from 0.002 to 1"
        ));
    }
    let finest = rules
        .min_width_mm
        .min(rules.min_clearance_mm)
        .min(rules.tool_mm);
    if res_mm > finest / 2.0 {
        return Err(format!(
            "the cell of {res_mm} mm is too coarse for the rules: it must be at most half of {finest} mm"
        ));
    }
    let mut out = Vec::new();
    for c in close_pieces(scene, rules.min_clearance_mm, res_mm)? {
        out.push(Violation {
            kind: Kind::Clearance,
            x: c.x,
            y: c.y,
            limit_mm: rules.min_clearance_mm,
        });
    }
    if rules.tool_mm > rules.min_clearance_mm {
        for c in close_pieces(scene, rules.tool_mm, res_mm)? {
            out.push(Violation {
                kind: Kind::ClearanceForTool,
                x: c.x,
                y: c.y,
                limit_mm: rules.tool_mm,
            });
        }
    }
    for (x, y) in narrow_copper(scene, rules.min_width_mm, res_mm)? {
        out.push(Violation {
            kind: Kind::Width,
            x,
            y,
            limit_mm: rules.min_width_mm,
        });
    }
    if !holes.is_empty() && (rules.annular_ring_mm > 0.0 || rules.holes_on_copper) {
        let bm = from_scene(scene, 2.0 * res_mm, res_mm)?;
        for h in holes {
            if h.diameter_mm <= 0.0 {
                if rules.annular_ring_mm > 0.0 {
                    return Err(format!(
                        "the hole at ({:.3}, {:.3}) has no diameter, so its ring cannot be checked",
                        h.x_mm, h.y_mm
                    ));
                }
                continue;
            }
            let on = bm.at(h.x_mm, h.y_mm) == Some(true);
            if !on {
                if rules.holes_on_copper {
                    out.push(Violation {
                        kind: Kind::HoleOffCopper,
                        x: h.x_mm,
                        y: h.y_mm,
                        limit_mm: h.diameter_mm,
                    });
                }
                continue;
            }
            // the circle just inside the outer edge of the ring must be all copper
            let radius = h.diameter_mm / 2.0 + rules.annular_ring_mm - res_mm;
            let steps =
                ((std::f64::consts::TAU * radius / res_mm).ceil() as usize).clamp(16, 20_000);
            let bad = (0..steps).any(|k| {
                let a = std::f64::consts::TAU * k as f64 / steps as f64;
                bm.at(h.x_mm + radius * a.cos(), h.y_mm + radius * a.sin()) != Some(true)
            });
            if bad && rules.annular_ring_mm > 0.0 {
                out.push(Violation {
                    kind: Kind::AnnularRing,
                    x: h.x_mm,
                    y: h.y_mm,
                    limit_mm: rules.annular_ring_mm,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        (a.kind, a.y, a.x)
            .partial_cmp(&(b.kind, b.y, b.x))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(out)
}
