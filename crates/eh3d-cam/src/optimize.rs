//! Lighter paths: spikes (a point where the tool goes out and comes straight back) are taken out, points repeated are joined and points that lie on a straight line, within a
//! tolerance, are dropped so that a straight stretch is one move.

use crate::isolate::Pass;
use eh3d_raster::contour::{simplify, simplify_open};

type Pt = (f64, f64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Optimize {
    /// No point of the original may be farther than this from the new path; 0 leaves the paths as they are.
    pub tolerance_mm: f64,
    /// A point whose shorter side is at most this long and where the path turns back is a spike; 0 leaves the spikes.
    pub spike_max_mm: f64,
    /// How sharply (the angle between the way in and the way out, 0 straight on to 180 back on itself) a point must turn back to be a spike.
    pub spike_angle_deg: f64,
}

impl Default for Optimize {
    fn default() -> Self {
        Optimize {
            tolerance_mm: 0.0,
            spike_max_mm: 0.0,
            spike_angle_deg: 170.0,
        }
    }
}

impl Optimize {
    pub fn validate(&self) -> Result<(), String> {
        let range = |name: &str, v: f64, lo: f64, hi: f64| {
            if v.is_finite() && v >= lo && v <= hi {
                Ok(())
            } else {
                Err(format!("{name} is {v}: it must be from {lo} to {hi}"))
            }
        };
        range("optimize_tolerance_mm", self.tolerance_mm, 0.0, 1.0)?;
        range("spike_max_mm", self.spike_max_mm, 0.0, 5.0)?;
        range("spike_angle_deg", self.spike_angle_deg, 90.0, 180.0)
    }

    pub fn is_off(&self) -> bool {
        self.tolerance_mm <= 0.0 && self.spike_max_mm <= 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub points_before: usize,
    pub points_after: usize,
    pub spikes: usize,
    /// Closed paths that were left with fewer than three points and dropped, open runs left with fewer than two.
    pub dropped_paths: usize,
}

fn dist(a: Pt, b: Pt) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The angle between the way in (a to b) and the way out (b to c), in degrees; 180 when the path turns back on itself.
fn turn_deg(a: Pt, b: Pt, c: Pt) -> f64 {
    let (u, v) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
    let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
    if lu < 1e-12 || lv < 1e-12 {
        return 0.0;
    }
    ((u.0 * v.0 + u.1 * v.1) / (lu * lv))
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

fn is_spike(a: Pt, b: Pt, c: Pt, o: &Optimize) -> bool {
    dist(a, b).min(dist(b, c)) <= o.spike_max_mm && turn_deg(a, b, c) >= o.spike_angle_deg
}

/// One path made lighter. Returns the points and how many spikes were taken out; a non-finite point is an error.
pub fn optimize_path(pts: &[Pt], closed: bool, o: &Optimize) -> Result<(Vec<Pt>, usize), String> {
    o.validate()?;
    if let Some(i) = pts
        .iter()
        .position(|p| !(p.0.is_finite() && p.1.is_finite()))
    {
        return Err(format!("point {}: a coordinate is not a number", i + 1));
    }
    // repeated points first
    let mut v: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if v.last().is_none_or(|l| dist(*l, *p) > 1e-9) {
            v.push(*p);
        }
    }
    if closed && v.len() > 1 && dist(v[0], v[v.len() - 1]) <= 1e-9 {
        v.pop();
    }
    let mut spikes = 0;
    if o.spike_max_mm > 0.0 {
        loop {
            let n = v.len();
            let min = if closed { 4 } else { 3 };
            if n < min {
                break;
            }
            let range: Vec<usize> = if closed {
                (0..n).collect()
            } else {
                (1..n - 1).collect()
            };
            let found = range
                .into_iter()
                .find(|&i| is_spike(v[(i + n - 1) % n], v[i], v[(i + 1) % n], o));
            match found {
                Some(i) => {
                    v.remove(i);
                    spikes += 1;
                }
                None => break,
            }
        }
    }
    if o.tolerance_mm > 0.0 {
        v = if closed {
            simplify(&v, o.tolerance_mm)
        } else {
            simplify_open(&v, o.tolerance_mm)
        };
    }
    Ok((v, spikes))
}

/// Makes every path of the passes lighter.
pub fn optimize_passes(passes: &mut [Pass], o: &Optimize) -> Result<Stats, String> {
    o.validate()?;
    let mut s = Stats::default();
    for (k, pass) in passes.iter_mut().enumerate() {
        for (list, closed) in [(&mut pass.paths, true), (&mut pass.open, false)] {
            let old = std::mem::take(list);
            for path in old {
                s.points_before += path.len();
                let (v, spikes) =
                    optimize_path(&path, closed, o).map_err(|e| format!("pass {}: {e}", k + 1))?;
                s.spikes += spikes;
                if v.len() < if closed { 3 } else { 2 } {
                    s.dropped_paths += 1;
                } else {
                    s.points_after += v.len();
                    list.push(v);
                }
            }
        }
    }
    Ok(s)
}
