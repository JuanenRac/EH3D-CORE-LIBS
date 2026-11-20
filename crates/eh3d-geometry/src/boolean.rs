//! Union, intersection, difference and exclusive or of regions with holes.
//!
//! The method: the points are moved to a grid of whole numbers (`BooleanOptions::grid`), every edge is cut where it
//! meets another edge with exact integer arithmetic, each piece is kept or dropped by asking whether its middle is
//! inside the other region, and the kept pieces are joined into rings. There is no dependency and no floating
//! point decision: whether a piece is inside is decided on whole numbers.
//!
//! # Meaning
//!
//! * The input regions follow the even-odd rule of [`crate::polygon`]: rings that are inside a ring are holes. Rings of
//!   one region must not cross each other or themselves; they may touch.
//! * Rings that are no wider than one grid step on average (slivers that the rounding leaves where two edges were meant
//!   to coincide) are dropped.
//! * The result is a region whose rings do not cross, with the outer rings counterclockwise and the holes
//!   clockwise (the filled side is always on the left), the corners that lie on a straight line removed, and the
//!   rings in an order that depends only on their points (the smallest point first), so the same input always gives
//!   the same output.
//! * Two regions that only touch along an edge are merged by a union (the shared edge disappears) and give no area in
//!   an intersection.
//!
//! # Precision
//!
//! A point is moved by at most half a grid step, and so is a crossing point; the area of the result differs from the
//! exact area by about the perimeter times the grid. With the default grid of one millionth of a unit that is far
//! below what a board needs. A coordinate whose distance to the origin is more than 2^40 grids is refused with
//! [`BooleanError::OutOfRange`]. The cost is quadratic in the number of edges that overlap in X; a pair of regions
//! with more than [`BooleanOptions::max_edges`] edges is refused with [`BooleanError::TooComplex`].

use crate::polygon::{Pt, Region};
use std::collections::BTreeMap;

/// What to do with two regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    /// Everything in either.
    Union,
    /// What is in both.
    Intersection,
    /// What is in the first and not in the second.
    Difference,
    /// What is in one and not in the other.
    Xor,
}

/// How the operations round and how far they go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BooleanOptions {
    /// The size of the grid the points are moved to, in the units of the points. Must be above zero.
    pub grid: f64,
    /// The most edges of the two regions together.
    pub max_edges: usize,
}

impl Default for BooleanOptions {
    fn default() -> Self {
        BooleanOptions {
            grid: 1e-6,
            max_edges: 20_000,
        }
    }
}

/// Why an operation was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanError {
    /// The grid is not a number above zero.
    BadGrid,
    /// A point is not finite or is farther from the origin than 2^40 grids.
    OutOfRange,
    /// The regions have more edges than `max_edges`.
    TooComplex,
    /// Some edges of the result could not be joined into closed rings, which happens when the rings of an input cross
    /// each other or themselves. [`combine`] returns the rings that did close and counts the others in
    /// [`Combined::dropped_edges`]; the four short functions refuse instead.
    Unjoinable,
}

impl std::fmt::Display for BooleanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            BooleanError::BadGrid => "the grid must be a number above zero",
            BooleanError::OutOfRange => {
                "a point is not finite or is too far from the origin for the grid"
            }
            BooleanError::TooComplex => "the regions have more edges than the limit",
            BooleanError::Unjoinable => {
                "edges of the result could not be joined into rings: the rings of an input cross"
            }
        })
    }
}

impl std::error::Error for BooleanError {}

/// The result of an operation and what was left out.
#[derive(Debug, Clone, PartialEq)]
pub struct Combined {
    /// The region.
    pub region: Region,
    /// How many edges could not be joined into a closed ring and were dropped. Zero for any input whose
    /// rings do not cross; above zero only when the input was not valid (or after a very rare rounding of a
    /// crossing).
    pub dropped_edges: usize,
}

type IPt = (i64, i64);

const LIMIT: i64 = 1 << 40;

fn to_grid(p: Pt, anchor: Pt, grid: f64) -> Result<IPt, BooleanError> {
    let (x, y) = ((p.0 - anchor.0) / grid, (p.1 - anchor.1) / grid);
    if !x.is_finite() || !y.is_finite() || x.abs() > LIMIT as f64 || y.abs() > LIMIT as f64 {
        return Err(BooleanError::OutOfRange);
    }
    Ok((x.round() as i64, y.round() as i64))
}

fn cross(o: IPt, a: IPt, b: IPt) -> i128 {
    (i128::from(a.0) - i128::from(o.0)) * (i128::from(b.1) - i128::from(o.1))
        - (i128::from(a.1) - i128::from(o.1)) * (i128::from(b.0) - i128::from(o.0))
}

fn ring_area2(ring: &[IPt]) -> i128 {
    let mut sum: i128 = 0;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        sum += i128::from(a.0) * i128::from(b.1) - i128::from(b.0) * i128::from(a.1);
    }
    sum
}

/// A point against all the rings by the even-odd rule, on doubled coordinates so the middle of an edge is exact.
/// A point exactly on an edge counts as outside (the callers only ask about the middle of a piece that no other
/// edge touches).
fn inside2(rings: &[Vec<IPt>], m: IPt) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let a = (ring[i].0 * 2, ring[i].1 * 2);
            let b = (ring[(i + 1) % n].0 * 2, ring[(i + 1) % n].1 * 2);
            if (a.1 > m.1) != (b.1 > m.1) {
                let c = cross(a, b, m);
                let upward = b.1 > a.1;
                if (c > 0) == upward && c != 0 {
                    inside = !inside;
                }
            }
        }
    }
    inside
}

/// Rings snapped to the grid, without repeated points, closing points or rings of no area, and oriented so the
/// filled side is on the left (outer rings counterclockwise, holes clockwise).
fn prepare(region: &Region, anchor: Pt, grid: f64) -> Result<Vec<Vec<IPt>>, BooleanError> {
    let mut rings: Vec<Vec<IPt>> = Vec::new();
    for ring in &region.rings {
        let mut pts: Vec<IPt> = Vec::with_capacity(ring.len());
        for p in ring {
            let q = to_grid(*p, anchor, grid)?;
            if pts.last() != Some(&q) {
                pts.push(q);
            }
        }
        while pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        if pts.len() >= 3 && ring_area2(&pts) != 0 {
            rings.push(pts);
        }
    }
    // the depth of each ring decides the way it turns
    let depths: Vec<usize> = rings
        .iter()
        .enumerate()
        .map(|(i, ring)| {
            let probe = probe_point(ring);
            rings
                .iter()
                .enumerate()
                .filter(|(j, other)| *j != i && inside2(std::slice::from_ref(*other), probe))
                .count()
        })
        .collect();
    for (ring, depth) in rings.iter_mut().zip(depths) {
        let ccw = ring_area2(ring) > 0;
        if ccw != (depth % 2 == 0) {
            ring.reverse();
        }
    }
    Ok(rings)
}

/// A point (in doubled coordinates) just inside a ring, from the middle of its longest edge nudged to the left, used
/// to ask which other rings hold this one. It is the middle of an edge of the ring itself, which no edge of a ring
/// that does not cross it can contain, so the answer is exact.
fn probe_point(ring: &[IPt]) -> IPt {
    let n = ring.len();
    let mut best = (0usize, -1i128);
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
        let len2 = dx * dx + dy * dy;
        if len2 > best.1 {
            best = (i, len2);
        }
    }
    let (a, b) = (ring[best.0], ring[(best.0 + 1) % n]);
    (a.0 + b.0, a.1 + b.1)
}

#[derive(Clone, Copy)]
struct Seg {
    a: IPt,
    b: IPt,
    source: u8,
}

/// The point where two segments that cross at one interior point meet, rounded to the grid, or the shared point
/// of two that touch.
fn split_points(s: &Seg, t: &Seg, out_s: &mut Vec<IPt>, out_t: &mut Vec<IPt>) {
    let (a, b, c, d) = (s.a, s.b, t.a, t.b);
    let d1 = cross(a, b, c);
    let d2 = cross(a, b, d);
    let d3 = cross(c, d, a);
    let d4 = cross(c, d, b);
    let strictly_inside = |p: IPt, u: IPt, v: IPt| {
        p != u
            && p != v
            && p.0 >= u.0.min(v.0)
            && p.0 <= u.0.max(v.0)
            && p.1 >= u.1.min(v.1)
            && p.1 <= u.1.max(v.1)
    };
    if d1 == 0 && d2 == 0 {
        // on one line: each end that is inside the other segment cuts it
        for p in [c, d] {
            if strictly_inside(p, a, b) {
                out_s.push(p);
            }
        }
        for p in [a, b] {
            if strictly_inside(p, c, d) {
                out_t.push(p);
            }
        }
        return;
    }
    if d1 == 0 && strictly_inside(c, a, b) {
        out_s.push(c);
    }
    if d2 == 0 && strictly_inside(d, a, b) {
        out_s.push(d);
    }
    if d3 == 0 && strictly_inside(a, c, d) {
        out_t.push(a);
    }
    if d4 == 0 && strictly_inside(b, c, d) {
        out_t.push(b);
    }
    if ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0)) && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0)) {
        // a proper crossing: a + (b - a) * t with t = (c - a) x (d - c) / (b - a) x (d - c)
        let (rx, ry) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
        let (sx, sy) = (i128::from(d.0 - c.0), i128::from(d.1 - c.1));
        let den = rx * sy - ry * sx;
        let num = (i128::from(c.0 - a.0)) * sy - (i128::from(c.1 - a.1)) * sx;
        if den != 0 {
            let round_div = |n: i128, d: i128| -> i128 {
                let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
                (2 * n + d).div_euclid(2 * d)
            };
            let x = i128::from(a.0) + round_div(rx * num, den);
            let y = i128::from(a.1) + round_div(ry * num, den);
            let p = (x as i64, y as i64);
            if p != a && p != b {
                out_s.push(p);
            }
            if p != c && p != d {
                out_t.push(p);
            }
        }
    }
}

/// Each corner and cut point mapped to the smallest point of the group of points that touch (one grid step apart
/// in either direction, counting steps of a chain).
fn cluster_map(segs: &[Seg], cuts: &[Vec<IPt>]) -> BTreeMap<IPt, IPt> {
    let mut pts: Vec<IPt> = segs
        .iter()
        .flat_map(|s| [s.a, s.b])
        .chain(cuts.iter().flatten().copied())
        .collect();
    pts.sort_unstable();
    pts.dedup();
    let index: BTreeMap<IPt, usize> = pts.iter().enumerate().map(|(i, p)| (*p, i)).collect();
    let mut parent: Vec<usize> = (0..pts.len()).collect();
    fn find(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for (i, p) in pts.iter().enumerate() {
        for dx in -1..=1i64 {
            for dy in -1..=1i64 {
                if (dx, dy) == (0, 0) {
                    continue;
                }
                if let Some(&j) = index.get(&(p.0 + dx, p.1 + dy)) {
                    let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                    if ri != rj {
                        // the smaller point is the root, so the answer does not depend on the order
                        let (lo, hi) = if pts[ri] < pts[rj] {
                            (ri, rj)
                        } else {
                            (rj, ri)
                        };
                        parent[hi] = lo;
                    }
                }
            }
        }
    }
    let mut out = BTreeMap::new();
    for (i, p) in pts.iter().enumerate() {
        let root = find(&mut parent, i);
        out.insert(*p, pts[root]);
    }
    out
}

fn sort_along(a: IPt, b: IPt, pts: &mut Vec<IPt>) {
    let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    pts.sort_by_key(|p| dx * i128::from(p.0 - a.0) + dy * i128::from(p.1 - a.1));
    pts.dedup();
}

/// Does `op` on two regions.
pub fn combine(
    op: Operation,
    a: &Region,
    b: &Region,
    options: &BooleanOptions,
) -> Result<Combined, BooleanError> {
    if options.grid.is_nan() || options.grid <= 0.0 || !options.grid.is_finite() {
        return Err(BooleanError::BadGrid);
    }
    // the points are moved to the grid round the middle of the two regions, so a board far from the origin keeps
    // its digits
    let all: Vec<Pt> = a
        .rings
        .iter()
        .chain(b.rings.iter())
        .flatten()
        .copied()
        .collect();
    let anchor = match crate::polygon::bounds(&all) {
        Some(bx) if bx.iter().all(|v| v.is_finite()) => {
            ((bx[0] + bx[2]) / 2.0, (bx[1] + bx[3]) / 2.0)
        }
        Some(_) => return Err(BooleanError::OutOfRange),
        None => (0.0, 0.0),
    };
    let ra = prepare(a, anchor, options.grid)?;
    let rb = prepare(b, anchor, options.grid)?;
    let edge_count: usize = ra.iter().chain(rb.iter()).map(Vec::len).sum();
    if edge_count > options.max_edges {
        return Err(BooleanError::TooComplex);
    }
    let mut segs: Vec<Seg> = Vec::with_capacity(edge_count);
    for (source, rings) in [(0u8, &ra), (1u8, &rb)] {
        for ring in rings {
            for i in 0..ring.len() {
                segs.push(Seg {
                    a: ring[i],
                    b: ring[(i + 1) % ring.len()],
                    source,
                });
            }
        }
    }
    // cut every edge where another edge meets it; the edges are looked at in order of their smallest x so the
    // search stops early
    let mut cuts: Vec<Vec<IPt>> = vec![Vec::new(); segs.len()];
    let mut order: Vec<usize> = (0..segs.len()).collect();
    order.sort_by_key(|&i| segs[i].a.0.min(segs[i].b.0));
    for (k, &i) in order.iter().enumerate() {
        let si = segs[i];
        let max_x = si.a.0.max(si.b.0);
        let (min_y, max_y) = (si.a.1.min(si.b.1), si.a.1.max(si.b.1));
        for &j in &order[k + 1..] {
            let sj = segs[j];
            if sj.a.0.min(sj.b.0) > max_x {
                break;
            }
            if sj.a.1.min(sj.b.1) > max_y || sj.a.1.max(sj.b.1) < min_y {
                continue;
            }
            let (mut ci, mut cj) = (Vec::new(), Vec::new());
            split_points(&si, &sj, &mut ci, &mut cj);
            cuts[i].extend(ci);
            cuts[j].extend(cj);
        }
    }
    // snap rounding: a corner of any ring that lies within one grid step of an edge cuts that edge, so edges that
    // were meant to meet at a point still meet at the same whole-number point and no sliver is left between them
    let mut corners: Vec<IPt> = segs.iter().map(|s| s.a).collect();
    corners.sort_unstable();
    corners.dedup();
    for (i, s) in segs.iter().enumerate() {
        let (lo, hi) = (s.a.0.min(s.b.0) - 1, s.a.0.max(s.b.0) + 1);
        let (ylo, yhi) = (s.a.1.min(s.b.1) - 1, s.a.1.max(s.b.1) + 1);
        let (dx, dy) = (i128::from(s.b.0 - s.a.0), i128::from(s.b.1 - s.a.1));
        let len2 = dx * dx + dy * dy;
        let first = corners.partition_point(|c| c.0 < lo);
        for &v in &corners[first..] {
            if v.0 > hi {
                break;
            }
            if v.1 < ylo || v.1 > yhi || v == s.a || v == s.b {
                continue;
            }
            let c = cross(s.a, s.b, v).abs();
            let along = dx * i128::from(v.0 - s.a.0) + dy * i128::from(v.1 - s.a.1);
            if c <= dx.abs() + dy.abs() && along > 0 && along < len2 {
                cuts[i].push(v);
            }
        }
    }
    // the pieces, each as a canonical (smaller point first) key with the direction it was written in
    struct Entry {
        net: [i32; 2],
    }
    let mut pieces: BTreeMap<(IPt, IPt), Entry> = BTreeMap::new();
    // points that are one grid step apart are the same point: every corner and every cut is moved to the
    // smallest point of its cluster, so two edges that meet near a point meet at exactly one point
    let snapped = cluster_map(&segs, &cuts);
    for (i, s) in segs.iter().enumerate() {
        let (a, b) = (snapped[&s.a], snapped[&s.b]);
        if a == b {
            continue;
        }
        let mut points: Vec<IPt> = std::mem::take(&mut cuts[i])
            .into_iter()
            .map(|p| snapped[&p])
            .collect();
        sort_along(a, b, &mut points);
        let mut chain = vec![a];
        chain.extend(points.into_iter().filter(|p| *p != a && *p != b));
        chain.push(b);
        for w in chain.windows(2) {
            let (p, q) = (w[0], w[1]);
            if p == q {
                continue;
            }
            let (key, dir) = if p < q { ((p, q), 1) } else { ((q, p), -1) };
            let e = pieces.entry(key).or_insert(Entry { net: [0, 0] });
            e.net[usize::from(s.source)] += dir;
        }
    }
    // keep or drop each piece
    let mut kept: Vec<(IPt, IPt)> = Vec::new();
    for ((p, q), e) in &pieces {
        let (da, db) = (e.net[0].signum(), e.net[1].signum());
        let mid = (p.0 + q.0, p.1 + q.1);
        let direction: i32; // 1 = from p to q
        let keep = match (da != 0, db != 0) {
            (true, true) => {
                if da == db {
                    direction = da;
                    matches!(op, Operation::Union | Operation::Intersection)
                } else {
                    direction = da;
                    op == Operation::Difference
                }
            }
            (true, false) => {
                let in_b = inside2(&rb, mid);
                match op {
                    Operation::Union | Operation::Difference => {
                        direction = da;
                        !in_b
                    }
                    Operation::Intersection => {
                        direction = da;
                        in_b
                    }
                    Operation::Xor => {
                        direction = if in_b { -da } else { da };
                        true
                    }
                }
            }
            (false, true) => {
                let in_a = inside2(&ra, mid);
                match op {
                    Operation::Union => {
                        direction = db;
                        !in_a
                    }
                    Operation::Intersection => {
                        direction = db;
                        in_a
                    }
                    Operation::Difference => {
                        direction = -db;
                        in_a
                    }
                    Operation::Xor => {
                        direction = if in_a { -db } else { db };
                        true
                    }
                }
            }
            (false, false) => {
                direction = 1;
                false
            }
        };
        if keep {
            kept.push(if direction > 0 { (*p, *q) } else { (*q, *p) });
        }
    }
    Ok(stitch(kept, anchor, options.grid))
}

fn turn_key(prev: IPt, at: IPt, next: IPt) -> f64 {
    // the angle to turn left from the way we came to the way we go, in (-pi, pi]
    let incoming = ((at.1 - prev.1) as f64).atan2((at.0 - prev.0) as f64);
    let outgoing = ((next.1 - at.1) as f64).atan2((next.0 - at.0) as f64);
    let mut t = outgoing - incoming;
    while t > std::f64::consts::PI {
        t -= std::f64::consts::TAU;
    }
    while t <= -std::f64::consts::PI {
        t += std::f64::consts::TAU;
    }
    t
}

fn stitch(edges: Vec<(IPt, IPt)>, anchor: Pt, grid: f64) -> Combined {
    let mut from: BTreeMap<IPt, Vec<usize>> = BTreeMap::new();
    for (i, (p, _)) in edges.iter().enumerate() {
        from.entry(*p).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut rings: Vec<Vec<IPt>> = Vec::new();
    let mut dropped = 0usize;
    for start in 0..edges.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut ring = vec![edges[start].0];
        let mut taken = vec![start];
        let mut current = start;
        let mut closed = false;
        loop {
            let (p, q) = edges[current];
            if q == edges[start].0 {
                closed = true;
                break;
            }
            ring.push(q);
            // the next edge: of those that start here and are free, the one that turns most to the right, which
            // keeps rings that only touch at a corner as separate rings
            let next = from.get(&q).and_then(|candidates| {
                candidates
                    .iter()
                    .copied()
                    .filter(|&c| !used[c])
                    .min_by(|&c1, &c2| {
                        turn_key(p, q, edges[c1].1)
                            .partial_cmp(&turn_key(p, q, edges[c2].1))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
            });
            match next {
                Some(n) => {
                    used[n] = true;
                    taken.push(n);
                    current = n;
                }
                None => break,
            }
        }
        if closed {
            rings.push(ring);
        } else {
            dropped += taken.len();
        }
    }
    // tidy: no corner on a straight line, no ring without area, a fixed starting point and order
    let mut out: Vec<Vec<Pt>> = Vec::new();
    for ring in rings {
        let ring = drop_straight(ring);
        if ring.len() < 3 || ring_area2(&ring) == 0 {
            continue;
        }
        // a ring that is on average no wider than one grid step is a sliver left by the rounding of two edges that
        // were meant to coincide, not a shape
        let perimeter: f64 = (0..ring.len())
            .map(|i| {
                let (p, q) = (ring[i], ring[(i + 1) % ring.len()]);
                ((q.0 - p.0) as f64).hypot((q.1 - p.1) as f64)
            })
            .sum();
        if (ring_area2(&ring).unsigned_abs() as f64) <= perimeter {
            continue;
        }
        let first = ring
            .iter()
            .enumerate()
            .min_by_key(|(_, p)| **p)
            .map_or(0, |(i, _)| i);
        let rotated: Vec<Pt> = ring
            .iter()
            .cycle()
            .skip(first)
            .take(ring.len())
            .map(|p| (anchor.0 + p.0 as f64 * grid, anchor.1 + p.1 as f64 * grid))
            .collect();
        out.push(rotated);
    }
    out.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.len().cmp(&b.len()))
    });
    Combined {
        region: Region { rings: out },
        dropped_edges: dropped,
    }
}

fn drop_straight(mut ring: Vec<IPt>) -> Vec<IPt> {
    loop {
        let n = ring.len();
        if n < 3 {
            return ring;
        }
        let mut removed = false;
        let mut i = 0;
        while i < ring.len() && ring.len() >= 3 {
            let m = ring.len();
            let (a, b, c) = (ring[(i + m - 1) % m], ring[i], ring[(i + 1) % m]);
            if cross(a, b, c) == 0 {
                ring.remove(i);
                removed = true;
            } else {
                i += 1;
            }
        }
        if !removed {
            return ring;
        }
    }
}

fn checked(c: Combined) -> Result<Region, BooleanError> {
    if c.dropped_edges > 0 {
        Err(BooleanError::Unjoinable)
    } else {
        Ok(c.region)
    }
}

/// The union of two regions.
pub fn union(a: &Region, b: &Region, options: &BooleanOptions) -> Result<Region, BooleanError> {
    combine(Operation::Union, a, b, options).and_then(checked)
}

/// What two regions have in common.
pub fn intersection(
    a: &Region,
    b: &Region,
    options: &BooleanOptions,
) -> Result<Region, BooleanError> {
    combine(Operation::Intersection, a, b, options).and_then(checked)
}

/// The first region without the second.
pub fn difference(
    a: &Region,
    b: &Region,
    options: &BooleanOptions,
) -> Result<Region, BooleanError> {
    combine(Operation::Difference, a, b, options).and_then(checked)
}

/// What is in one of the regions and not in the other.
pub fn xor(a: &Region, b: &Region, options: &BooleanOptions) -> Result<Region, BooleanError> {
    combine(Operation::Xor, a, b, options).and_then(checked)
}

/// The union of many regions, joined two by two as a tree so no region is joined to a very large one more than
/// needed.
pub fn union_all(regions: &[Region], options: &BooleanOptions) -> Result<Region, BooleanError> {
    let mut level: Vec<Region> = regions.to_vec();
    if level.is_empty() {
        return Ok(Region::new());
    }
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            match pair {
                [a, b] => next.push(union(a, b, options)?),
                [a] => next.push(a.clone()),
                _ => {}
            }
        }
        level = next;
    }
    // a single region still goes through the same clean-up as the others
    let only = level.remove(0);
    union(&only, &Region::new(), options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> BooleanOptions {
        BooleanOptions::default()
    }

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn two_overlapping_squares_in_the_four_operations() {
        let a = Region::rectangle(0.0, 0.0, 4.0, 4.0);
        let b = Region::rectangle(2.0, 2.0, 6.0, 6.0);
        assert!(near(union(&a, &b, &opts()).unwrap().area(), 28.0));
        assert!(near(intersection(&a, &b, &opts()).unwrap().area(), 4.0));
        assert!(near(difference(&a, &b, &opts()).unwrap().area(), 12.0));
        assert!(near(difference(&b, &a, &opts()).unwrap().area(), 12.0));
        assert!(near(xor(&a, &b, &opts()).unwrap().area(), 24.0));
        let u = union(&a, &b, &opts()).unwrap();
        assert_eq!(u.rings.len(), 1);
        assert_eq!(
            u.rings[0].len(),
            8,
            "the L-shaped outline has eight corners"
        );
    }

    #[test]
    fn a_square_inside_another_makes_a_hole_in_a_difference_and_vanishes_in_a_union() {
        let big = Region::rectangle(0.0, 0.0, 10.0, 10.0);
        let small = Region::rectangle(3.0, 3.0, 5.0, 5.0);
        let hole = difference(&big, &small, &opts()).unwrap();
        assert_eq!(hole.rings.len(), 2);
        assert!(near(hole.area(), 96.0));
        let outer = hole
            .rings
            .iter()
            .find(|r| r.len() == 4 && crate::polygon::signed_area(r) > 0.0);
        let inner = hole
            .rings
            .iter()
            .find(|r| crate::polygon::signed_area(r) < 0.0);
        assert!(
            outer.is_some() && inner.is_some(),
            "outer counterclockwise, hole clockwise"
        );
        assert!(near(union(&big, &small, &opts()).unwrap().area(), 100.0));
        assert!(near(
            intersection(&big, &small, &opts()).unwrap().area(),
            4.0
        ));
        assert!(difference(&small, &big, &opts()).unwrap().is_empty());
    }

    #[test]
    fn a_region_with_a_hole_can_be_cut_by_a_bar_through_the_hole() {
        let mut ring = Region::rectangle(0.0, 0.0, 10.0, 10.0);
        ring.rings
            .push(vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)]);
        assert!(near(ring.area(), 84.0));
        let bar = Region::rectangle(-1.0, 4.0, 11.0, 6.0);
        let cut = difference(&ring, &bar, &opts()).unwrap();
        // the bar removes the sides of the frame at that height: two pieces, the top and the bottom
        assert!(near(cut.area(), 84.0 - (10.0 * 2.0 - 4.0 * 2.0)));
        assert_eq!(cut.rings.len(), 2, "{cut:?}");
        let both = intersection(&ring, &bar, &opts()).unwrap();
        assert!(near(both.area(), 12.0));
        assert_eq!(
            both.rings.len(),
            2,
            "two bars of the frame, one at each side of the hole"
        );
    }

    #[test]
    fn regions_that_touch_along_an_edge_are_merged_and_have_nothing_in_common() {
        let a = Region::rectangle(0.0, 0.0, 2.0, 2.0);
        let b = Region::rectangle(2.0, 0.0, 4.0, 2.0);
        let u = union(&a, &b, &opts()).unwrap();
        assert_eq!(u.rings.len(), 1);
        assert_eq!(
            u.rings[0].len(),
            4,
            "the shared edge and its corners disappear"
        );
        assert!(near(u.area(), 8.0));
        assert!(intersection(&a, &b, &opts()).unwrap().is_empty());
        assert!(near(difference(&a, &b, &opts()).unwrap().area(), 4.0));
        assert!(xor(&a, &b, &opts()).unwrap().rings.len() == 1);
        // touching at one corner only
        let c = Region::rectangle(2.0, 2.0, 4.0, 4.0);
        let corner = union(&a, &c, &opts()).unwrap();
        assert!(near(corner.area(), 8.0));
        assert!(intersection(&a, &c, &opts()).unwrap().is_empty());
    }

    #[test]
    fn identical_regions_and_empty_regions() {
        let a = Region::rectangle(0.0, 0.0, 3.0, 3.0);
        let empty = Region::new();
        assert!(near(union(&a, &a, &opts()).unwrap().area(), 9.0));
        assert!(near(intersection(&a, &a, &opts()).unwrap().area(), 9.0));
        assert!(difference(&a, &a, &opts()).unwrap().is_empty());
        assert!(xor(&a, &a, &opts()).unwrap().is_empty());
        assert!(near(union(&a, &empty, &opts()).unwrap().area(), 9.0));
        assert!(intersection(&a, &empty, &opts()).unwrap().is_empty());
        assert!(near(difference(&a, &empty, &opts()).unwrap().area(), 9.0));
        assert!(difference(&empty, &a, &opts()).unwrap().is_empty());
        assert!(union(&empty, &empty, &opts()).unwrap().is_empty());
    }

    #[test]
    fn the_orientation_of_the_input_rings_does_not_matter() {
        let a = Region::rectangle(0.0, 0.0, 4.0, 4.0);
        let mut b = Region::rectangle(2.0, 2.0, 6.0, 6.0);
        b.rings[0].reverse();
        assert!(near(union(&a, &b, &opts()).unwrap().area(), 28.0));
        let mut holey = Region::rectangle(0.0, 0.0, 10.0, 10.0);
        holey
            .rings
            .push(vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0)]);
        let mut flipped = holey.clone();
        flipped.rings[1].reverse();
        let probe = Region::rectangle(5.0, 5.0, 12.0, 12.0);
        assert_eq!(
            difference(&holey, &probe, &opts()).unwrap(),
            difference(&flipped, &probe, &opts()).unwrap()
        );
    }

    #[test]
    fn the_same_input_always_gives_the_same_output() {
        let a = Region::regular_polygon((0.0, 0.0), 5.0, 17);
        let b = Region::regular_polygon((2.0, 1.0), 4.0, 11);
        let first = union(&a, &b, &opts()).unwrap();
        for _ in 0..5 {
            assert_eq!(union(&a, &b, &opts()).unwrap(), first);
        }
        // and the starting point of every ring is its smallest point
        for ring in &first.rings {
            let min = ring
                .iter()
                .copied()
                .fold(ring[0], |m, p| if p < m { p } else { m });
            assert_eq!(ring[0], min);
        }
    }

    #[test]
    fn the_options_and_the_input_are_checked() {
        let a = Region::rectangle(0.0, 0.0, 1.0, 1.0);
        let bad = BooleanOptions {
            grid: 0.0,
            ..opts()
        };
        assert_eq!(union(&a, &a, &bad), Err(BooleanError::BadGrid));
        let nan = BooleanOptions {
            grid: f64::NAN,
            ..opts()
        };
        assert_eq!(union(&a, &a, &nan), Err(BooleanError::BadGrid));
        let far = Region::rectangle(0.0, 0.0, 1.0e9, 1.0);
        assert_eq!(union(&a, &far, &opts()), Err(BooleanError::OutOfRange));
        let nan_pt = Region::from_ring(vec![(0.0, 0.0), (f64::NAN, 1.0), (1.0, 0.0)]);
        assert_eq!(union(&a, &nan_pt, &opts()), Err(BooleanError::OutOfRange));
        let small = BooleanOptions {
            max_edges: 5,
            ..opts()
        };
        assert_eq!(union(&a, &a, &small), Err(BooleanError::TooComplex));
        assert!(BooleanError::TooComplex.to_string().contains("limit"));
        assert!(BooleanError::Unjoinable.to_string().contains("cross"));
    }

    #[test]
    fn the_union_of_many_pieces_is_the_union_one_by_one() {
        let pieces: Vec<Region> = (0..9)
            .map(|i| Region::rectangle(f64::from(i), 0.0, f64::from(i) + 1.5, 1.0))
            .collect();
        let all = union_all(&pieces, &opts()).unwrap();
        assert!(near(all.area(), 9.5));
        assert_eq!(all.rings.len(), 1);
        assert!(union_all(&[], &opts()).unwrap().is_empty());
        assert!(near(union_all(&pieces[..1], &opts()).unwrap().area(), 1.5));
    }

    #[test]
    fn far_from_the_origin_the_result_keeps_its_precision() {
        let far = 1.5e6;
        let a = Region::rectangle(far, far, far + 4.0, far + 4.0);
        let b = Region::rectangle(far + 2.0, far + 2.0, far + 6.0, far + 6.0);
        assert!(near(union(&a, &b, &opts()).unwrap().area(), 28.0));
        assert!(near(intersection(&a, &b, &opts()).unwrap().area(), 4.0));
    }
}
