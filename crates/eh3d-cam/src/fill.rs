//! Clearing by contours: parallel lines of a tool that fill the inside of closed paths (even-odd, so a path inside another one is a hole) and, when the copper is given, only the
//! stretches where the tool stays on copper. Like the clearing of a box, nothing is smoothed at the edges: this is for the copper that is left after the passes are cut.

use crate::bitmap::Bitmap;
use crate::isolate::{Pass, MAX_RUBOUT_LINES};
use crate::transform::Axis;
use eh3d_geometry::scene::Scene;
use eh3d_raster::edt::distance_mm;

type Pt = (f64, f64);

/// What the lines are like.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    pub tool_mm: f64,
    /// The share of the width of the tool that neighbouring lines have in common, 0 to 0.9.
    pub overlap: f64,
    pub direction: Axis,
    /// How much farther than the radius of the tool a line keeps from the edge of the copper (when the copper is given); negative is not allowed.
    pub margin_mm: f64,
    /// The size of the cell the copper is drawn on.
    pub res_mm: f64,
}

/// The even-odd crossings of the line `y = yc` with the contours, sorted.
fn crossings(contours: &[Vec<Pt>], yc: f64) -> Vec<f64> {
    let mut xs = Vec::new();
    for c in contours {
        let n = c.len();
        for i in 0..n {
            let (a, b) = (c[i], c[(i + 1) % n]);
            if (a.1 > yc) != (b.1 > yc) {
                xs.push(a.0 + (yc - a.1) * (b.0 - a.0) / (b.1 - a.1));
            }
        }
    }
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    xs
}

/// Fills the inside of the contours with lines of a tool, clipped to the copper when `copper` is given: the centre of the tool is kept where the tool lies wholly on copper (its
/// radius plus the margin from every cell that is not copper). The lines are open runs that follow each other back and forth.
pub fn rubout_contours(
    contours: &[Vec<Pt>],
    copper: Option<&Scene>,
    f: &Fill,
) -> Result<Pass, String> {
    if !(f.tool_mm.is_finite() && (0.1..=10.0).contains(&f.tool_mm)) {
        return Err(format!(
            "the tool of the lines is {} mm: it must be from 0.1 to 10",
            f.tool_mm
        ));
    }
    if !(f.overlap.is_finite() && (0.0..=0.9).contains(&f.overlap)) {
        return Err(format!(
            "the overlap is {}: it must be from 0 to 0.9",
            f.overlap
        ));
    }
    if !(f.margin_mm.is_finite() && (0.0..=10.0).contains(&f.margin_mm)) {
        return Err(format!(
            "the margin is {} mm: it must be from 0 to 10",
            f.margin_mm
        ));
    }
    let closed: Vec<&Vec<Pt>> = contours.iter().filter(|c| c.len() >= 3).collect();
    if closed.is_empty() {
        return Err("there is no closed contour to fill".into());
    }
    if closed
        .iter()
        .flat_map(|c| c.iter())
        .any(|p| !(p.0.is_finite() && p.1.is_finite()))
    {
        return Err("a contour has a coordinate that is not a number".into());
    }
    // work with horizontal lines; for vertical ones the two coordinates trade places on the way in and on the way out
    let swap = |p: Pt| -> Pt {
        if f.direction == Axis::Vertical {
            (p.1, p.0)
        } else {
            p
        }
    };
    let shapes: Vec<Vec<Pt>> = closed
        .iter()
        .map(|c| c.iter().map(|p| swap(*p)).collect())
        .collect();
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in shapes.iter().flatten() {
        y0 = y0.min(p.1);
        y1 = y1.max(p.1);
    }
    let r = f.tool_mm / 2.0;
    let across = y1 - y0;
    if across < f.tool_mm {
        return Err(format!(
            "the contours are {across:.3} mm across the lines: at least the tool of {} mm is needed",
            f.tool_mm
        ));
    }
    let pitch = f.tool_mm * (1.0 - f.overlap);
    let steps = ((across - f.tool_mm) / pitch).ceil().max(0.0) as usize;
    if steps + 1 > MAX_RUBOUT_LINES {
        return Err(format!(
            "{} lines: at most {MAX_RUBOUT_LINES} (use a wider tool or a smaller contour)",
            steps + 1
        ));
    }
    let step = if steps == 0 {
        0.0
    } else {
        (across - f.tool_mm) / steps as f64
    };
    // how far inside the copper every cell is (the distance to the nearest cell that is not copper), on the grid of the copper
    let grid: Option<(Bitmap, Vec<f32>)> = match copper {
        Some(sc) => {
            let mut bm = Bitmap::new(&sc.bounds(), r + f.margin_mm + 4.0 * f.res_mm, f.res_mm)?;
            bm.draw(sc);
            let off: Vec<bool> = bm.on.iter().map(|c| !*c).collect();
            let d = distance_mm(&off, bm.w, bm.h, bm.res);
            Some((bm, d))
        }
        None => None,
    };
    let need = r + f.margin_mm - f.res_mm;
    let allowed = |x: f64, y: f64| -> bool {
        match &grid {
            None => true,
            Some((bm, d)) => {
                let (px, py) = if f.direction == Axis::Vertical {
                    (y, x)
                } else {
                    (x, y)
                };
                bm.index_at(px, py)
                    .is_some_and(|i| bm.on[i] && f64::from(d[i]) >= need)
            }
        }
    };
    let mut open: Vec<Vec<Pt>> = Vec::new();
    for k in 0..=steps {
        let yc = y0 + r + k as f64 * step;
        let xs = crossings(&shapes, yc);
        let mut runs: Vec<(f64, f64)> = Vec::new();
        for i in (0..xs.len().saturating_sub(1)).step_by(2) {
            let (a, b) = (xs[i] + r, xs[i + 1] - r);
            if b < a {
                continue;
            }
            match &grid {
                None => runs.push((a, b)),
                Some(_) => {
                    let n = (((b - a) / f.res_mm).ceil() as usize).max(1);
                    let mut start: Option<f64> = None;
                    let mut last = a;
                    for j in 0..=n {
                        let x = a + (b - a) * j as f64 / n as f64;
                        if allowed(x, yc) {
                            if start.is_none() {
                                start = Some(x);
                            }
                            last = x;
                        } else if let Some(s) = start.take() {
                            if last - s >= f.res_mm {
                                runs.push((s, last));
                            }
                        }
                    }
                    if let Some(s) = start {
                        if last - s >= f.res_mm {
                            runs.push((s, last));
                        }
                    }
                }
            }
        }
        // back and forth: every other line is walked the other way, and so are the runs of one line
        if k % 2 == 1 {
            runs.reverse();
        }
        for (a, b) in runs {
            let (from, to) = if k % 2 == 0 { (a, b) } else { (b, a) };
            open.push(vec![swap((from, yc)), swap((to, yc))]);
        }
    }
    if open.is_empty() {
        return Err(
            "no line of the tool fits inside the contours (and on the copper): nothing to clear"
                .into(),
        );
    }
    Ok(Pass {
        offset_mm: 0.0,
        open,
        diameter_mm: f.tool_mm,
        rubout: true,
        ..Pass::default()
    })
}
