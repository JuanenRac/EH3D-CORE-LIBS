//! The line where the distance to the copper equals a level: marching squares on the grid of distances, the segments joined into closed paths, and the paths made lighter with the
//! Douglas-Peucker rule (no point of the original is farther than `tolerance_mm` from the result).

use std::collections::{HashMap, HashSet};

/// A closed path in mm (the last point is not repeated).
pub type Loop = Vec<(f64, f64)>;

type Edge = (usize, usize, u8);

pub struct Grid<'a> {
    pub dist: &'a [f32],
    pub w: usize,
    pub h: usize,
    pub res: f64,
    pub x0: f64,
    pub y0: f64,
}

impl Grid<'_> {
    fn d(&self, ix: usize, iy: usize, level: f64) -> f64 {
        f64::from(self.dist[iy * self.w + ix]).min(3.0 * level)
    }

    fn point(&self, e: Edge, level: f64) -> (f64, f64) {
        let (ix, iy, kind) = e;
        let (jx, jy) = if kind == 0 {
            (ix + 1, iy)
        } else {
            (ix, iy + 1)
        };
        let (d0, d1) = (self.d(ix, iy, level), self.d(jx, jy, level));
        let t = if (d1 - d0).abs() < 1e-12 {
            0.5
        } else {
            ((level - d0) / (d1 - d0)).clamp(0.0, 1.0)
        };
        let (x, y) = (
            self.x0 + (ix as f64 + 0.5 + t * (jx as f64 - ix as f64)) * self.res,
            self.y0 + (iy as f64 + 0.5 + t * (jy as f64 - iy as f64)) * self.res,
        );
        (x, y)
    }
}

/// The closed paths where the distance is `level_mm`.
pub fn loops(g: &Grid, level_mm: f64) -> Vec<Loop> {
    let inside = |ix: usize, iy: usize| f64::from(g.dist[iy * g.w + ix]) < level_mm;
    let mut adj: HashMap<Edge, Vec<Edge>> = HashMap::new();
    let mut link = |a: Edge, b: Edge| {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    };
    for iy in 0..g.h.saturating_sub(1) {
        for ix in 0..g.w.saturating_sub(1) {
            let b = [
                inside(ix, iy),
                inside(ix + 1, iy),
                inside(ix + 1, iy + 1),
                inside(ix, iy + 1),
            ];
            if b.iter().all(|v| *v) || b.iter().all(|v| !*v) {
                continue;
            }
            // the edges of the cell: 0 top, 1 right, 2 bottom, 3 left (between the corners 0-1, 1-2, 3-2, 0-3)
            let edges: [Edge; 4] = [(ix, iy, 0), (ix + 1, iy, 1), (ix, iy + 1, 0), (ix, iy, 1)];
            let crossing: Vec<usize> = (0..4)
                .filter(|&e| b[[0, 1, 3, 0][e]] != b[[1, 2, 2, 3][e]])
                .collect();
            if crossing.len() == 2 {
                link(edges[crossing[0]], edges[crossing[1]]);
            } else if crossing.len() == 4 {
                let centre = (g.d(ix, iy, level_mm)
                    + g.d(ix + 1, iy, level_mm)
                    + g.d(ix + 1, iy + 1, level_mm)
                    + g.d(ix, iy + 1, level_mm))
                    / 4.0
                    < level_mm;
                // the corners 0 and 2 are alike, and 1 and 3 are alike: the centre decides which pair is joined
                let around_one_and_three = (b[0] && centre) || (!b[0] && !centre);
                if around_one_and_three {
                    link(edges[0], edges[1]);
                    link(edges[2], edges[3]);
                } else {
                    link(edges[0], edges[3]);
                    link(edges[1], edges[2]);
                }
            }
        }
    }
    let mut seen: HashSet<Edge> = HashSet::new();
    let mut keys: Vec<Edge> = adj.keys().copied().collect();
    keys.sort_unstable();
    let mut out = Vec::new();
    for start in keys {
        if seen.contains(&start) {
            continue;
        }
        let (mut path, mut cur, mut prev) = (Vec::new(), start, None::<Edge>);
        loop {
            seen.insert(cur);
            path.push(g.point(cur, level_mm));
            let next = adj[&cur]
                .iter()
                .copied()
                .find(|n| Some(*n) != prev && !seen.contains(n));
            match next {
                Some(n) => {
                    prev = Some(cur);
                    cur = n;
                }
                None => break,
            }
        }
        if path.len() >= 3 {
            out.push(path);
        }
    }
    out
}

/// Douglas-Peucker on a closed path: the farthest pair of points splits it in two open lines that are simplified.
pub fn simplify(path: &[(f64, f64)], tolerance_mm: f64) -> Loop {
    if path.len() < 4 || tolerance_mm <= 0.0 {
        return path.to_vec();
    }
    let far = (1..path.len())
        .max_by(|&a, &b| {
            dist2(path[0], path[a])
                .partial_cmp(&dist2(path[0], path[b]))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(1);
    let mut a = open(&path[..=far], tolerance_mm);
    let mut closing: Vec<(f64, f64)> = path[far..].to_vec();
    closing.push(path[0]);
    let b = open(&closing, tolerance_mm);
    a.pop();
    a.extend(b);
    a.pop();
    a
}

fn dist2(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)
}

fn line_distance(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dy);
    if len < 1e-12 {
        return dist2(p, a).sqrt();
    }
    ((p.0 - a.0) * dy - (p.1 - a.1) * dx).abs() / len
}

fn open(points: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
    let n = points.len();
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[n - 1] = true;
    let mut stack = vec![(0usize, n - 1)];
    while let Some((a, b)) = stack.pop() {
        let mut worst = (0.0, a);
        for i in a + 1..b {
            let d = line_distance(points[i], points[a], points[b]);
            if d > worst.0 {
                worst = (d, i);
            }
        }
        if worst.0 > tol {
            keep[worst.1] = true;
            stack.push((a, worst.1));
            stack.push((worst.1, b));
        }
    }
    points
        .iter()
        .zip(&keep)
        .filter(|(_, k)| **k)
        .map(|(p, _)| *p)
        .collect()
}

/// Douglas-Peucker on an open run of points (the ends stay).
pub fn simplify_open(points: &[(f64, f64)], tolerance_mm: f64) -> Vec<(f64, f64)> {
    if points.len() < 3 || tolerance_mm <= 0.0 {
        return points.to_vec();
    }
    open(points, tolerance_mm)
}

impl Grid<'_> {
    /// The distance at a point of the board (the nearest cell), or `None` outside the grid.
    pub fn sample(&self, x: f64, y: f64) -> Option<f64> {
        let (fx, fy) = (
            (x - self.x0) / self.res - 0.5,
            (y - self.y0) / self.res - 0.5,
        );
        let (ix, iy) = (fx.round(), fy.round());
        if ix < 0.0 || iy < 0.0 || ix >= self.w as f64 || iy >= self.h as f64 {
            return None;
        }
        Some(f64::from(self.dist[iy as usize * self.w + ix as usize]))
    }
}

/// The most edges of a path that `orient_climb` looks at.
const CLIMB_EDGES: usize = 64;

/// Turns a path round when needed so that the copper is on the right of the tool (the tool goes clockwise round an island of copper). The side is found by looking at the
/// distance to the copper a little to each side of the edges of the path: it is smaller on the side of the copper. The edges vote, each with its length, so a path between two
/// pieces of copper (where the longest edge alone has the same distance on both sides) goes the way of the nearer one; a path with no side nearer than the other is not turned.
pub fn orient_climb(g: &Grid, path: &mut Loop) {
    let n = path.len();
    if n < 3 {
        return;
    }
    // every edge when there are few, and the longest ones when there are many
    let mut edges: Vec<(usize, f64)> = (0..n)
        .map(|i| {
            let (a, b) = (path[i], path[(i + 1) % n]);
            (i, (b.0 - a.0).hypot(b.1 - a.1))
        })
        .filter(|(_, l)| *l >= 1e-9)
        .collect();
    if edges.is_empty() {
        return;
    }
    if edges.len() > CLIMB_EDGES {
        edges.sort_by(|p, q| q.1.partial_cmp(&p.1).unwrap_or(std::cmp::Ordering::Equal));
        edges.truncate(CLIMB_EDGES);
    }
    let step = 1.5 * g.res;
    // positive: the copper is on the right
    let mut score = 0.0;
    for (i, len) in edges {
        let (a, b) = (path[i], path[(i + 1) % n]);
        let m = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let (nx, ny) = ((b.1 - a.1) / len, -(b.0 - a.0) / len);
        let right = g.sample(m.0 + nx * step, m.1 + ny * step);
        let left = g.sample(m.0 - nx * step, m.1 - ny * step);
        score += len
            * match (right, left) {
                (Some(r), Some(l)) => l - r,
                (Some(_), None) => step,
                (None, Some(_)) => -step,
                (None, None) => 0.0,
            };
    }
    if score < -1e-9 {
        path.reverse();
    }
}
