//! The passes of the tool around the copper, and the places where two pieces of copper are too close for the tool to go between them.

use crate::bitmap::{from_scene, Bitmap};
use crate::order::Cut;
use crate::transform::Axis;
use eh3d_geometry::scene::{Scene, Shape};
use eh3d_raster::contour::{loops, orient_climb, simplify, simplify_open, Grid, Loop};
use eh3d_raster::edt::distance_mm;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Params {
    /// The width of the tool at the depth of the cut.
    pub tool_diameter_mm: f64,
    /// How many passes, each one a stepover farther from the copper than the one before.
    pub passes: usize,
    pub stepover_mm: f64,
    /// The size of a cell of the grid: the paths are within this of the exact ones.
    pub res_mm: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            tool_diameter_mm: 0.2,
            passes: 1,
            stepover_mm: 0.1,
            res_mm: 0.02,
        }
    }
}

impl Params {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("tool_diameter_mm", self.tool_diameter_mm, 0.02, 10.0)?;
        range("stepover_mm", self.stepover_mm, 0.01, 10.0)?;
        range("res_mm", self.res_mm, 0.002, 1.0)?;
        if !(1..=10).contains(&self.passes) {
            return Err(format!(
                "passes is {}: it must be from 1 to 10",
                self.passes
            ));
        }
        if self.res_mm > self.tool_diameter_mm / 2.0 {
            return Err(format!("the cell of {} mm is too coarse for a tool of {} mm: it must be at most half of it", self.res_mm, self.tool_diameter_mm));
        }
        Ok(())
    }

    /// The farthest the tool goes from the copper, centre of the tool to the edge of the copper.
    pub fn reach_mm(&self) -> f64 {
        self.tool_diameter_mm / 2.0 + (self.passes - 1) as f64 * self.stepover_mm
    }
}

/// One tool of the isolation: its width and how many passes it makes, each one a stepover farther from the copper.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tool {
    pub diameter_mm: f64,
    pub passes: usize,
    pub stepover_mm: f64,
}

impl Tool {
    /// The distance from the copper to the centre of the tool in its last pass.
    pub fn last_offset_mm(&self) -> f64 {
        self.diameter_mm / 2.0 + (self.passes.max(1) - 1) as f64 * self.stepover_mm
    }
}

/// The removal of the copper that is left over beyond the passes of isolation: rings of a wider tool, each one overlapping the one before.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rubout {
    pub tool_mm: f64,
    /// How far from the copper the rings go, measured to the edge of the cut.
    pub width_mm: f64,
    /// The part of the width of the tool that a ring shares with the one before (0 to 0.9).
    pub overlap: f64,
}

/// Everything the isolation of a layer needs: one tool or several (the widest cuts first), the optional removal of the leftover copper and the size of the cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub tools: Vec<Tool>,
    pub rubout: Option<Rubout>,
    pub res_mm: f64,
    /// Leave out the stretches of a narrower tool that a wider one has already cut along.
    pub trim: bool,
}

/// The most rings of the removal of the leftover copper.
pub const MAX_RUBOUT_RINGS: usize = 200;

impl Plan {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        if !(1..=4).contains(&self.tools.len()) {
            return Err(format!(
                "{} tools: there must be from 1 to 4",
                self.tools.len()
            ));
        }
        range("res_mm", self.res_mm, 0.002, 1.0)?;
        let mut passes = 0;
        for t in &self.tools {
            range("tool_diameter_mm", t.diameter_mm, 0.02, 10.0)?;
            range("stepover_mm", t.stepover_mm, 0.01, 10.0)?;
            if !(1..=10).contains(&t.passes) {
                return Err(format!("passes is {}: it must be from 1 to 10", t.passes));
            }
            if self.res_mm > t.diameter_mm / 2.0 {
                return Err(format!("the cell of {} mm is too coarse for a tool of {} mm: it must be at most half of it", self.res_mm, t.diameter_mm));
            }
            passes += t.passes;
        }
        if passes > 40 {
            return Err(format!("{passes} passes in all: at most 40"));
        }
        if let Some(r) = &self.rubout {
            range("rubout_tool_mm", r.tool_mm, 0.1, 10.0)?;
            range("rubout_width_mm", r.width_mm, 0.05, 50.0)?;
            range("rubout_overlap", r.overlap, 0.0, 0.9)?;
            if self.res_mm > r.tool_mm / 2.0 {
                return Err(format!("the cell of {} mm is too coarse for a tool of {} mm: it must be at most half of it", self.res_mm, r.tool_mm));
            }
        }
        Ok(())
    }

    /// The narrowest tool, which is the one that has to fit between the pieces of copper.
    pub fn finest_mm(&self) -> f64 {
        self.tools
            .iter()
            .map(|t| t.diameter_mm)
            .fold(f64::INFINITY, f64::min)
    }

    /// The farthest the grid has to reach from the copper.
    pub fn reach_mm(&self) -> f64 {
        let tools = self
            .tools
            .iter()
            .map(|t| t.last_offset_mm() + t.diameter_mm / 2.0)
            .fold(0.0, f64::max);
        match &self.rubout {
            Some(r) => tools.max(r.width_mm + r.tool_mm * 1.5),
            None => tools,
        }
    }
}

impl Params {
    /// The same isolation as a plan with one tool.
    pub fn plan(&self) -> Plan {
        Plan {
            tools: vec![Tool {
                diameter_mm: self.tool_diameter_mm,
                passes: self.passes,
                stepover_mm: self.stepover_mm,
            }],
            rubout: None,
            res_mm: self.res_mm,
            trim: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pass {
    /// The distance from the edge of the copper to the centre of the tool.
    pub offset_mm: f64,
    /// The closed paths.
    pub paths: Vec<Loop>,
    /// Runs that are not closed: what is left of a path of a narrower tool where a wider one has cut already.
    pub open: Vec<Loop>,
    /// The tool (its place in the plan; the rings of the removal of leftover copper come after the tools).
    pub tool: usize,
    /// The width of the tool; 0 when it is not known.
    pub diameter_mm: f64,
    /// True for a ring of the removal of leftover copper.
    pub rubout: bool,
}

impl Pass {
    /// Everything this pass cuts, closed paths first.
    pub fn cuts(&self) -> Vec<Cut> {
        self.paths
            .iter()
            .filter(|p| p.len() >= 3)
            .map(|p| Cut {
                pts: p.clone(),
                closed: true,
            })
            .chain(self.open.iter().filter(|p| p.len() >= 2).map(|p| Cut {
                pts: p.clone(),
                closed: false,
            }))
            .collect()
    }
}

/// The passes, from the first one (the tool touching the copper) outwards.
pub fn isolate(scene: &Scene, p: &Params) -> Result<Vec<Pass>, String> {
    p.validate()?;
    plan(scene, &p.plan())
}

fn mark(bm: &Bitmap, mask: &mut [bool], pts: &[(f64, f64)], closed: bool) {
    let n = pts.len();
    let segs = if closed { n } else { n.saturating_sub(1) };
    for i in 0..segs {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let steps = ((b.0 - a.0).hypot(b.1 - a.1) / (bm.res / 2.0))
            .ceil()
            .max(1.0) as usize;
        for k in 0..=steps {
            let t = k as f64 / steps as f64;
            if let Some(ix) = bm.index_at(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t) {
                mask[ix] = true;
            }
        }
    }
}

/// The points of a closed path with none farther than `step` from the next.
fn resample(path: &[(f64, f64)], step: f64) -> Vec<(f64, f64)> {
    let n = path.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (path[i], path[(i + 1) % n]);
        let parts = ((b.0 - a.0).hypot(b.1 - a.1) / step).ceil().max(1.0) as usize;
        for k in 0..parts {
            let t = k as f64 / parts as f64;
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    out
}

/// What is left of a closed path where `covered` says the ground has been cut already: the path itself when none of it is, nothing when all of it is, otherwise the open runs
/// that are not (each run takes one covered point at each end so that no gap is left).
fn trim_path(
    path: &[(f64, f64)],
    covered: &dyn Fn((f64, f64)) -> bool,
    res: f64,
) -> (Option<Loop>, Vec<Loop>) {
    let pts = resample(path, res);
    let flags: Vec<bool> = pts.iter().map(|p| covered(*p)).collect();
    if flags.iter().all(|c| !*c) {
        return (Some(path.to_vec()), Vec::new());
    }
    if flags.iter().all(|c| *c) {
        return (None, Vec::new());
    }
    let n = pts.len();
    let first = (0..n)
        .find(|&i| flags[i] && !flags[(i + n - 1) % n])
        .unwrap_or(0);
    let (mut runs, mut cur) = (Vec::new(), Vec::new());
    for j in 0..n {
        let i = (first + j) % n;
        if !flags[i] {
            if cur.is_empty() {
                cur.push(pts[(i + n - 1) % n]);
            }
            cur.push(pts[i]);
        } else if !cur.is_empty() {
            cur.push(pts[i]);
            runs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        cur.push(pts[first]);
        runs.push(cur);
    }
    let runs = runs
        .into_iter()
        .map(|r| simplify_open(&r, res / 2.0))
        .filter(|r| r.len() >= 2)
        .collect();
    (None, runs)
}

/// The passes of every tool of the plan, the widest tool first, and then the rings that remove the leftover copper.
pub fn plan(scene: &Scene, p: &Plan) -> Result<Vec<Pass>, String> {
    plan_with_pads(scene, p, None)
}

/// The smaller side of a pad (a disc, a rectangle or an obround), or `None` for tracks and regions.
fn pad_size_mm(s: &Shape) -> Option<f64> {
    match s {
        Shape::Disc { r, .. } => Some(2.0 * r),
        Shape::Rect { w, h, .. } | Shape::Obround { w, h, .. } => Some(w.min(*h)),
        _ => None,
    }
}

/// Like `plan`, but with `around_pads_min_mm` set every tool except the narrowest one cuts only the rings round pads whose smaller side is at least that wide; the narrowest tool
/// does the rest (tracks and small pads), and with `trim` it leaves out what the wider tools have cut.
pub fn plan_with_pads(
    scene: &Scene,
    p: &Plan,
    around_pads_min_mm: Option<f64>,
) -> Result<Vec<Pass>, String> {
    p.validate()?;
    if let Some(m) = around_pads_min_mm {
        if !(m.is_finite() && m > 0.0 && m <= 50.0) {
            return Err(format!(
                "around_pads_min_mm is {m}: it must be from 0 (exclusive) to 50"
            ));
        }
    }
    let res = p.res_mm;
    let bm = from_scene(scene, p.reach_mm() + 4.0 * res, res)?;
    let dist = distance_mm(&bm.on, bm.w, bm.h, bm.res);
    // the distance to the big pads alone, on the same grid
    let pad_dist: Option<Vec<f32>> = around_pads_min_mm.map(|m| {
        let mut pads = bm.clone();
        pads.on.iter_mut().for_each(|c| *c = false);
        let only = Scene {
            shapes: scene
                .shapes
                .iter()
                .filter(|s| pad_size_mm(s).is_some_and(|d| d >= m))
                .cloned()
                .collect(),
            notes: Vec::new(),
        };
        pads.draw(&only);
        if pads.count() == 0 {
            vec![f32::INFINITY; pads.w * pads.h]
        } else {
            distance_mm(&pads.on, pads.w, pads.h, pads.res)
        }
    });
    let grid = Grid {
        dist: &dist,
        w: bm.w,
        h: bm.h,
        res: bm.res,
        x0: bm.x0,
        y0: bm.y0,
    };
    // the distance is measured between centres of cells, which puts the edge of the copper half a cell away
    let traced = |offset: f64| -> Vec<Loop> {
        loops(&grid, offset + res / 2.0)
            .into_iter()
            .map(|l| simplify(&l, res / 2.0))
            .filter(|l| l.len() >= 3)
            .map(|mut l| {
                orient_climb(&grid, &mut l);
                l
            })
            .collect()
    };
    let mut order: Vec<usize> = (0..p.tools.len()).collect();
    order.sort_by(|&a, &b| {
        p.tools[b]
            .diameter_mm
            .partial_cmp(&p.tools[a].diameter_mm)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut cut = vec![false; bm.w * bm.h];
    let mut out = Vec::new();
    for (rank, &ti) in order.iter().enumerate() {
        let t = &p.tools[ti];
        let mut path_cells = vec![false; bm.w * bm.h];
        for k in 0..t.passes {
            let offset = t.diameter_mm / 2.0 + k as f64 * t.stepover_mm;
            let mut pass = Pass {
                offset_mm: offset,
                tool: ti,
                diameter_mm: t.diameter_mm,
                ..Pass::default()
            };
            for l in traced(offset) {
                if let (Some(pd), true) = (&pad_dist, rank + 1 < order.len()) {
                    // a wider tool: only where the path runs round a big pad; the rest is left to the finest tool
                    let reach = offset + 2.0 * res;
                    let not_pad = |q: (f64, f64)| {
                        bm.index_at(q.0, q.1)
                            .is_none_or(|ix| f64::from(pd[ix]) > reach)
                    };
                    let (whole, runs) = trim_path(&l, &not_pad, res);
                    if let Some(w) = whole {
                        mark(&bm, &mut path_cells, &w, true);
                        pass.paths.push(w);
                    }
                    for r in runs {
                        mark(&bm, &mut path_cells, &r, false);
                        pass.open.push(r);
                    }
                } else if p.trim && rank > 0 {
                    let covered = |q: (f64, f64)| bm.index_at(q.0, q.1).is_some_and(|ix| cut[ix]);
                    let (whole, runs) = trim_path(&l, &covered, res);
                    if let Some(w) = whole {
                        mark(&bm, &mut path_cells, &w, true);
                        pass.paths.push(w);
                    }
                    for r in runs {
                        mark(&bm, &mut path_cells, &r, false);
                        pass.open.push(r);
                    }
                } else {
                    mark(&bm, &mut path_cells, &l, true);
                    pass.paths.push(l);
                }
            }
            out.push(pass);
        }
        if p.trim && rank + 1 < order.len() {
            // the ground this tool has cut: within its radius of its paths (less two cells, so that a stretch is only left out when it is surely cut)
            let near = distance_mm(&path_cells, bm.w, bm.h, bm.res);
            let reach = (t.diameter_mm / 2.0 - 2.0 * res).max(0.0);
            for (c, d) in cut.iter_mut().zip(&near) {
                if f64::from(*d) <= reach {
                    *c = true;
                }
            }
        }
    }
    if let Some(r) = &p.rubout {
        let edge = p
            .tools
            .iter()
            .map(|t| t.last_offset_mm() + t.diameter_mm / 2.0)
            .fold(0.0, f64::max);
        let rr = r.tool_mm / 2.0;
        let step = r.tool_mm * (1.0 - r.overlap);
        let first = rr.max(edge + rr - r.tool_mm * r.overlap);
        for k in 0..MAX_RUBOUT_RINGS {
            let offset = first + k as f64 * step;
            if offset - rr >= r.width_mm {
                break;
            }
            out.push(Pass {
                offset_mm: offset,
                paths: traced(offset),
                tool: p.tools.len(),
                diameter_mm: r.tool_mm,
                rubout: true,
                ..Pass::default()
            });
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Closeness {
    /// Where the pieces come close, in mm.
    pub x: f64,
    pub y: f64,
    /// How many separate pieces of copper are closer than the limit there.
    pub pieces: usize,
}

/// Labels the connected cells of a grid (8 neighbours): the number of each cell's piece (0 for no copper) and how many pieces there are.
pub fn label(on: &[bool], w: usize, h: usize) -> (Vec<u32>, u32) {
    let (mut id, mut n, mut stack) = (vec![0u32; w * h], 0u32, Vec::new());
    for s in 0..w * h {
        if !on[s] || id[s] != 0 {
            continue;
        }
        n += 1;
        id[s] = n;
        stack.push(s);
        while let Some(i) = stack.pop() {
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            for (dx, dy) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                let (nx, ny) = (x + dx, y + dy);
                if nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h {
                    let j = ny as usize * w + nx as usize;
                    if on[j] && id[j] == 0 {
                        id[j] = n;
                        stack.push(j);
                    }
                }
            }
        }
    }
    (id, n)
}

/// The places where separate pieces of copper are closer than `min_gap_mm`: a tool as wide as that gap cannot go between them, so the isolation would cut into the copper or leave a bridge.
pub fn close_pieces(scene: &Scene, min_gap_mm: f64, res_mm: f64) -> Result<Vec<Closeness>, String> {
    if !(min_gap_mm.is_finite() && min_gap_mm > 0.0) {
        return Err(format!(
            "min_gap_mm is {min_gap_mm}: it must be more than 0"
        ));
    }
    let bm = from_scene(scene, min_gap_mm + 4.0 * res_mm, res_mm)?;
    let (pieces, count) = label(&bm.on, bm.w, bm.h);
    if count < 2 {
        return Ok(Vec::new());
    }
    let dist = distance_mm(&bm.on, bm.w, bm.h, bm.res);
    // every cell within half of the gap of the copper joins the pieces that are closer than the gap
    let reach = min_gap_mm / 2.0 + res_mm / 2.0;
    let grown: Vec<bool> = dist.iter().map(|d| f64::from(*d) <= reach).collect();
    let (merged, merged_count) = label(&grown, bm.w, bm.h);
    // one pass over the grid, gathering what each merged piece holds (going through the whole grid
    // once per merged piece made the time grow with pieces x cells)
    struct Group {
        originals: Vec<u32>,
        sx: f64,
        sy: f64,
        cells: usize,
    }
    let mut groups: Vec<Group> = (0..merged_count)
        .map(|_| Group {
            originals: Vec::new(),
            sx: 0.0,
            sy: 0.0,
            cells: 0,
        })
        .collect();
    for (i, &m) in merged.iter().enumerate() {
        if m == 0 {
            continue;
        }
        let g = &mut groups[m as usize - 1];
        g.sx += (i % bm.w) as f64;
        g.sy += (i / bm.w) as f64;
        g.cells += 1;
        let piece = pieces[i];
        if piece != 0 && g.originals.last() != Some(&piece) {
            g.originals.push(piece);
        }
    }
    let mut out = Vec::new();
    for mut g in groups {
        g.originals.sort_unstable();
        g.originals.dedup();
        if g.originals.len() >= 2 {
            let (x, y) = bm.centre(g.sx / g.cells as f64, g.sy / g.cells as f64);
            out.push(Closeness {
                x,
                y,
                pieces: g.originals.len(),
            });
        }
    }
    Ok(out)
}

/// The label of the copper at a point, looking at the cell itself and the eight round it (a pad smaller than a cell may leave its own centre cell empty).
fn label_near(bm: &Bitmap, ids: &[u32], x: f64, y: f64) -> Option<u32> {
    let i = bm.index_at(x, y)?;
    let (cx, cy) = ((i % bm.w) as isize, (i / bm.w) as isize);
    for (dx, dy) in [
        (0, 0),
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ] {
        let (nx, ny) = (cx + dx, cy + dy);
        if nx >= 0 && ny >= 0 && (nx as usize) < bm.w && (ny as usize) < bm.h {
            let id = ids[ny as usize * bm.w + nx as usize];
            if id != 0 {
                return Some(id);
            }
        }
    }
    None
}

/// How far from the chosen point the copper of a net is looked for.
pub const NET_SEARCH_MM: f64 = 0.5;

/// The copper of one net: the shapes of the scene that are joined to the copper at (or within `NET_SEARCH_MM` of) the point `at`. Everything else is left out, so a plan made
/// from the result isolates that net alone, with whichever tool the plan has.
pub fn net_scene(scene: &Scene, at: (f64, f64), res_mm: f64) -> Result<Scene, String> {
    if !(at.0.is_finite() && at.1.is_finite()) {
        return Err("the point of the net is not a number".into());
    }
    let bm = from_scene(scene, 2.0 * res_mm, res_mm)?;
    let (ids, _) = label(&bm.on, bm.w, bm.h);
    // the nearest copper cell to the point, within the search distance
    let reach = (NET_SEARCH_MM / res_mm).ceil() as isize;
    let centre = bm
        .index_at(at.0, at.1)
        .ok_or_else(|| format!("the point ({}, {}) is not on the layer", at.0, at.1))?;
    let (cx, cy) = ((centre % bm.w) as isize, (centre / bm.w) as isize);
    let mut best: Option<(f64, u32)> = None;
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let (nx, ny) = (cx + dx, cy + dy);
            if nx < 0 || ny < 0 || nx as usize >= bm.w || ny as usize >= bm.h {
                continue;
            }
            let id = ids[ny as usize * bm.w + nx as usize];
            let d = ((dx * dx + dy * dy) as f64).sqrt() * res_mm;
            if id != 0 && d <= NET_SEARCH_MM && best.is_none_or(|b| d < b.0) {
                best = Some((d, id));
            }
        }
    }
    let (_, net) = best.ok_or_else(|| {
        format!(
            "there is no copper within {NET_SEARCH_MM} mm of ({}, {})",
            at.0, at.1
        )
    })?;
    let on_net = |x: f64, y: f64| label_near(&bm, &ids, x, y) == Some(net);
    let shapes: Vec<Shape> = scene
        .shapes
        .iter()
        .filter(|s| match s {
            Shape::Disc { x, y, .. } | Shape::Rect { x, y, .. } | Shape::Obround { x, y, .. } => {
                on_net(*x, *y)
            }
            Shape::Stroke { x0, y0, x1, y1, .. } => {
                on_net(*x0, *y0) || on_net(*x1, *y1) || on_net((x0 + x1) / 2.0, (y0 + y1) / 2.0)
            }
            Shape::Region { pts } => pts.iter().any(|p| on_net(p.0, p.1)),
        })
        .cloned()
        .collect();
    if shapes.is_empty() {
        return Err("the net holds no shape that can be drawn".into());
    }
    Ok(Scene {
        shapes,
        notes: scene.notes.clone(),
    })
}

/// The most lines of the clearing of an area.
pub const MAX_RUBOUT_LINES: usize = 5000;

/// Clears an area (`x0, y0, x1, y1`) with parallel lines of a tool: the lines run along `direction` (horizontal lines for `Axis::Horizontal`), the tool reaches the edge of the
/// area on every side, and neighbouring lines share `overlap` of the width of the tool (0 to 0.9) or a little more, so that the last one fits the area exactly. The lines are
/// open runs that follow each other back and forth. Nothing is smoothed at the edges of the copper: this is for the leftover copper after the passes are cut.
pub fn rubout_lines(
    area: (f64, f64, f64, f64),
    tool_mm: f64,
    overlap: f64,
    direction: Axis,
) -> Result<Pass, String> {
    let (x0, y0, x1, y1) = (
        area.0.min(area.2),
        area.1.min(area.3),
        area.0.max(area.2),
        area.1.max(area.3),
    );
    if ![x0, y0, x1, y1].iter().all(|v| v.is_finite()) {
        return Err("the area is not a number".into());
    }
    if !(tool_mm.is_finite() && (0.1..=10.0).contains(&tool_mm)) {
        return Err(format!(
            "the tool of the lines is {tool_mm} mm: it must be from 0.1 to 10"
        ));
    }
    if !(overlap.is_finite() && (0.0..=0.9).contains(&overlap)) {
        return Err(format!(
            "the overlap is {overlap}: it must be from 0 to 0.9"
        ));
    }
    let (along, across) = match direction {
        Axis::Horizontal => (x1 - x0, y1 - y0),
        Axis::Vertical => (y1 - y0, x1 - x0),
    };
    if along <= tool_mm || across < tool_mm {
        return Err(format!(
            "the area is {} by {} mm: it must be wider than the tool of {tool_mm} mm along the lines and at least as wide across them",
            x1 - x0,
            y1 - y0
        ));
    }
    let pitch = tool_mm * (1.0 - overlap);
    let steps = ((across - tool_mm) / pitch).ceil().max(0.0) as usize;
    if steps + 1 > MAX_RUBOUT_LINES {
        return Err(format!(
            "{} lines: at most {MAX_RUBOUT_LINES} (use a wider tool or a smaller area)",
            steps + 1
        ));
    }
    let r = tool_mm / 2.0;
    let step = if steps == 0 {
        0.0
    } else {
        (across - tool_mm) / steps as f64
    };
    let mut open: Vec<Loop> = Vec::new();
    for k in 0..=steps {
        let c = k as f64 * step + r;
        let (a, b) = (r, along - r);
        let (from, to) = if k % 2 == 0 { (a, b) } else { (b, a) };
        open.push(match direction {
            Axis::Horizontal => vec![(x0 + from, y0 + c), (x0 + to, y0 + c)],
            Axis::Vertical => vec![(x0 + c, y0 + from), (x0 + c, y0 + to)],
        });
    }
    Ok(Pass {
        offset_mm: 0.0,
        open,
        diameter_mm: tool_mm,
        rubout: true,
        ..Pass::default()
    })
}
