//! Forced isolation: where two pieces of copper are closer than the finest tool is wide, the normal paths go round both and leave the gap uncut. With this switched on, a path is
//! also laid along the middle of every such gap, as long as the gap is not narrower than a minimum; the gaps that are narrower are listed and nothing is laid there.
//! The tool is wider than the gap there, so it will touch the copper on both sides: every place is reported with its coordinates and the gap.

use crate::bitmap::from_scene;
use crate::isolate::{label, Pass, Plan};
use eh3d_geometry::scene::Scene;
use eh3d_raster::contour::simplify_open;
use eh3d_raster::edt::distance_mm;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Forced {
    /// Gaps narrower than this get no path.
    pub min_gap_mm: f64,
}

impl Forced {
    pub fn validate(&self, plan: &Plan) -> Result<(), String> {
        if !(self.min_gap_mm.is_finite() && self.min_gap_mm >= 0.02 && self.min_gap_mm <= 10.0) {
            return Err(format!(
                "forced_min_gap_mm is {}: it must be from 0.02 to 10",
                self.min_gap_mm
            ));
        }
        if self.min_gap_mm < 2.0 * plan.res_mm {
            return Err(format!(
                "forced_min_gap_mm is {}: it must be at least two cells ({} mm)",
                self.min_gap_mm,
                2.0 * plan.res_mm
            ));
        }
        if self.min_gap_mm >= plan.finest_mm() {
            return Err(format!(
                "forced_min_gap_mm is {}: it must be less than the finest tool ({} mm), or no gap is forced",
                self.min_gap_mm,
                plan.finest_mm()
            ));
        }
        Ok(())
    }
}

/// A place where the tool does not fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Site {
    pub x: f64,
    pub y: f64,
    /// The narrowest gap there, in mm.
    pub gap_mm: f64,
    /// The length of the path laid there (0 for a gap that is too narrow), in mm.
    pub length_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ForcedResult {
    /// The pass to add (empty when there is nothing to force).
    pub passes: Vec<Pass>,
    pub forced: Vec<Site>,
    pub too_narrow: Vec<Site>,
}

const NEIGHBOURS: [(isize, isize); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Zhang-Suen thinning of a set of cells (marked in `on`) down to lines one cell wide.
fn thin(on: &mut [bool], cells: &mut Vec<usize>, w: usize, h: usize) {
    let at = |on: &[bool], i: usize, dx: isize, dy: isize| -> bool {
        let (x, y) = ((i % w) as isize + dx, (i / w) as isize + dy);
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && on[y as usize * w + x as usize]
    };
    loop {
        let mut changed = false;
        for step in 0..2 {
            let mut remove = Vec::new();
            for &i in cells.iter() {
                // p2..p9 clockwise from the cell above
                let p = [
                    at(on, i, 0, -1),
                    at(on, i, 1, -1),
                    at(on, i, 1, 0),
                    at(on, i, 1, 1),
                    at(on, i, 0, 1),
                    at(on, i, -1, 1),
                    at(on, i, -1, 0),
                    at(on, i, -1, -1),
                ];
                let b = p.iter().filter(|v| **v).count();
                let a = (0..8).filter(|&k| !p[k] && p[(k + 1) % 8]).count();
                let ok = if step == 0 {
                    !(p[0] && p[2] && p[4]) && !(p[2] && p[4] && p[6])
                } else {
                    !(p[0] && p[2] && p[6]) && !(p[0] && p[4] && p[6])
                };
                if (2..=6).contains(&b) && a == 1 && ok {
                    remove.push(i);
                }
            }
            if !remove.is_empty() {
                changed = true;
                for &i in &remove {
                    on[i] = false;
                }
                cells.retain(|&i| on[i]);
            }
        }
        if !changed {
            break;
        }
    }
}

/// Walks the lines of a thinned set of cells into paths of cell indices.
fn walk(on: &[bool], cells: &[usize], w: usize, h: usize) -> Vec<Vec<usize>> {
    let nb = |i: usize| -> Vec<usize> {
        let (x, y) = ((i % w) as isize, (i / w) as isize);
        NEIGHBOURS
            .iter()
            .filter_map(|(dx, dy)| {
                let (nx, ny) = (x + dx, y + dy);
                (nx >= 0 && ny >= 0 && (nx as usize) < w && (ny as usize) < h)
                    .then(|| ny as usize * w + nx as usize)
            })
            .filter(|&j| on[j])
            .collect()
    };
    let mut seen: HashSet<usize> = HashSet::new();
    let mut order: Vec<usize> = cells.to_vec();
    // ends first, then the cells with the fewest neighbours
    order.sort_by_cached_key(|&i| (nb(i).len(), i));
    let mut out = Vec::new();
    for s in order {
        if seen.contains(&s) {
            continue;
        }
        let mut path = Vec::new();
        // a line that branches off one already walked starts from the cell it joins
        if let Some(j) = nb(s).into_iter().find(|j| seen.contains(j)) {
            path.push(j);
        }
        let mut cur = s;
        loop {
            seen.insert(cur);
            path.push(cur);
            let (cx, cy) = ((cur % w) as isize, (cur / w) as isize);
            let next = nb(cur)
                .into_iter()
                .filter(|j| !seen.contains(j))
                .min_by_key(|&j| {
                    let (dx, dy) = ((j % w) as isize - cx, (j / w) as isize - cy);
                    (dx.abs() + dy.abs(), j)
                });
            match next {
                Some(n) => cur = n,
                None => break,
            }
        }
        out.push(path);
    }
    out
}

/// Groups cells that touch (8 neighbours) and gives the centre and the narrowest gap of each group.
fn groups(gaps: &HashMap<usize, f32>, w: usize, x0: f64, y0: f64, res: f64) -> Vec<Site> {
    let mut seen: HashSet<usize> = HashSet::new();
    let mut keys: Vec<usize> = gaps.keys().copied().collect();
    keys.sort_unstable();
    let mut out = Vec::new();
    for s in keys {
        if seen.contains(&s) {
            continue;
        }
        let (mut stack, mut cells) = (vec![s], Vec::new());
        seen.insert(s);
        while let Some(i) = stack.pop() {
            cells.push(i);
            let (x, y) = ((i % w) as isize, (i / w) as isize);
            for (dx, dy) in NEIGHBOURS {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if (nx as usize) < w && gaps.contains_key(&j) && seen.insert(j) {
                    stack.push(j);
                }
            }
        }
        let n = cells.len() as f64;
        let (sx, sy) = cells.iter().fold((0.0, 0.0), |a, &i| {
            (a.0 + (i % w) as f64, a.1 + (i / w) as f64)
        });
        let gap = cells
            .iter()
            .map(|i| f64::from(gaps[i]))
            .fold(f64::INFINITY, f64::min);
        out.push(Site {
            x: x0 + (sx / n + 0.5) * res,
            y: y0 + (sy / n + 0.5) * res,
            gap_mm: gap,
            length_mm: 0.0,
        });
    }
    out
}

/// The paths along the middle of the gaps that the finest tool of the plan does not fit in, and the gaps that are too narrow even for these.
pub fn forced_passes(scene: &Scene, plan: &Plan, f: &Forced) -> Result<ForcedResult, String> {
    plan.validate()?;
    f.validate(plan)?;
    let res = plan.res_mm;
    let d = plan.finest_mm();
    let tool = plan
        .tools
        .iter()
        .position(|t| (t.diameter_mm - d).abs() < 1e-12)
        .unwrap_or(0);
    let bm = from_scene(scene, d + 4.0 * res, res)?;
    let (pieces, count) = label(&bm.on, bm.w, bm.h);
    if count < 2 {
        return Ok(ForcedResult::default());
    }
    let dist = distance_mm(&bm.on, bm.w, bm.h, bm.res);
    // the cells within half of the tool of the copper, joined: only the groups that hold two pieces can hold a gap
    let reach = d / 2.0 + res / 2.0;
    let grown: Vec<bool> = dist.iter().map(|v| f64::from(*v) <= reach).collect();
    let (merged, merged_count) = label(&grown, bm.w, bm.h);
    let mut first = vec![0u32; merged_count as usize + 1];
    let mut multi = vec![false; merged_count as usize + 1];
    for i in 0..bm.w * bm.h {
        if bm.on[i] {
            let m = merged[i] as usize;
            if first[m] == 0 {
                first[m] = pieces[i];
            } else if first[m] != pieces[i] {
                multi[m] = true;
            }
        }
    }
    let k = (d / res).ceil() as isize + 1;
    let (w, h) = (bm.w as isize, bm.h as isize);
    let mut usable: HashMap<usize, f32> = HashMap::new();
    let mut narrow: HashMap<usize, f32> = HashMap::new();
    for i in 0..bm.w * bm.h {
        if bm.on[i] || !grown[i] || !multi[merged[i] as usize] {
            continue;
        }
        let (cx, cy) = ((i % bm.w) as isize, (i / bm.w) as isize);
        // the nearest piece, and the nearest other piece
        let mut near: Vec<(u32, f64)> = Vec::new();
        for dy in -k..=k {
            let y = cy + dy;
            if y < 0 || y >= h {
                continue;
            }
            for dx in -k..=k {
                let x = cx + dx;
                if x < 0 || x >= w {
                    continue;
                }
                let j = y as usize * bm.w + x as usize;
                if !bm.on[j] {
                    continue;
                }
                let dd = ((dx * dx + dy * dy) as f64).sqrt() * res;
                match near.iter_mut().find(|(p, _)| *p == pieces[j]) {
                    Some(e) => e.1 = e.1.min(dd),
                    None => near.push((pieces[j], dd)),
                }
            }
        }
        if near.len() < 2 {
            continue;
        }
        near.sort_by(|a, b| a.1.total_cmp(&b.1));
        // the distances are between centres of cells: one cell is the extra
        let gap = near[0].1 + near[1].1 - res;
        if gap < d - 1e-9 {
            if gap >= f.min_gap_mm {
                usable.insert(i, gap as f32);
            } else {
                narrow.insert(i, gap.max(0.0) as f32);
            }
        }
    }
    let mut out = ForcedResult {
        too_narrow: groups(&narrow, bm.w, bm.x0, bm.y0, res),
        ..ForcedResult::default()
    };
    if usable.is_empty() {
        return Ok(out);
    }
    let mut on = vec![false; bm.w * bm.h];
    let mut cells: Vec<usize> = usable.keys().copied().collect();
    cells.sort_unstable();
    for &i in &cells {
        on[i] = true;
    }
    thin(&mut on, &mut cells, bm.w, bm.h);
    let mut open = Vec::new();
    for path in walk(&on, &cells, bm.w, bm.h) {
        let pts: Vec<(f64, f64)> = path
            .iter()
            .map(|&i| bm.centre((i % bm.w) as f64, (i / bm.w) as f64))
            .collect();
        let pts = simplify_open(&pts, res / 2.0);
        let length: f64 = pts
            .windows(2)
            .map(|p| (p[1].0 - p[0].0).hypot(p[1].1 - p[0].1))
            .sum();
        if pts.len() < 2 || length < 2.0 * res {
            continue;
        }
        let gap = path
            .iter()
            .filter_map(|i| usable.get(i))
            .map(|g| f64::from(*g))
            .fold(f64::INFINITY, f64::min);
        let mid = pts[pts.len() / 2];
        out.forced.push(Site {
            x: mid.0,
            y: mid.1,
            gap_mm: gap,
            length_mm: length,
        });
        open.push(pts);
    }
    if !open.is_empty() {
        let narrowest = out
            .forced
            .iter()
            .map(|s| s.gap_mm)
            .fold(f64::INFINITY, f64::min);
        out.passes.push(Pass {
            offset_mm: narrowest / 2.0,
            paths: Vec::new(),
            open,
            tool,
            diameter_mm: d,
            rubout: false,
        });
    }
    Ok(out)
}

/// Puts the forced pass after the passes of the tools and before the rings that clear the leftover copper.
pub fn insert(passes: &mut Vec<Pass>, extra: Vec<Pass>) {
    let at = passes.iter().position(|p| p.rubout).unwrap_or(passes.len());
    for (k, p) in extra.into_iter().enumerate() {
        passes.insert(at + k, p);
    }
}
