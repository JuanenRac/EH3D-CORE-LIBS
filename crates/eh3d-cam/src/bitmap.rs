//! The copper drawn on a grid of square cells of `res` mm: a cell is copper when its centre is inside a pad, a track or a region.

use eh3d_geometry::scene::{Bounds, Scene, Shape};

/// The most cells a grid may have (a board of 200 x 200 mm at 0.02 mm is 100 million: too many; the tool says so instead of running out of memory).
pub const MAX_CELLS: usize = 60_000_000;

#[derive(Debug, Clone, PartialEq)]
pub struct Bitmap {
    pub w: usize,
    pub h: usize,
    pub res: f64,
    /// The lower left corner of the grid, in mm.
    pub x0: f64,
    pub y0: f64,
    pub on: Vec<bool>,
}

impl Bitmap {
    pub fn new(bounds: &Bounds, margin_mm: f64, res: f64) -> Result<Bitmap, String> {
        if !(res.is_finite() && (0.002..=1.0).contains(&res)) {
            return Err(format!("the cell is {res} mm: it must be from 0.002 to 1"));
        }
        if bounds.is_empty() {
            return Err("the layer has no copper".into());
        }
        let m = margin_mm.max(0.0);
        let (x0, y0) = (bounds.min_x - m, bounds.min_y - m);
        // counted as numbers first: a size out of range must not wrap on its way to a count
        let wf = ((bounds.width() + 2.0 * m) / res).ceil() + 1.0;
        let hf = ((bounds.height() + 2.0 * m) / res).ceil() + 1.0;
        if !(wf.is_finite() && hf.is_finite()) || wf * hf > MAX_CELLS as f64 {
            return Err(format!("{wf} x {hf} cells is more than the {MAX_CELLS} allowed: use a larger cell or a smaller board"));
        }
        let (w, h) = (wf as usize, hf as usize);
        Ok(Bitmap {
            w,
            h,
            res,
            x0,
            y0,
            on: vec![false; w * h],
        })
    }

    /// The centre of the cell `(ix, iy)` in mm.
    pub fn centre(&self, ix: f64, iy: f64) -> (f64, f64) {
        (
            self.x0 + (ix + 0.5) * self.res,
            self.y0 + (iy + 0.5) * self.res,
        )
    }

    /// The index of the cell that holds a point of the board, or `None` outside the grid.
    pub fn index_at(&self, x: f64, y: f64) -> Option<usize> {
        let (fx, fy) = (
            ((x - self.x0) / self.res).floor(),
            ((y - self.y0) / self.res).floor(),
        );
        if !(fx >= 0.0 && fy >= 0.0 && fx < self.w as f64 && fy < self.h as f64) {
            return None;
        }
        Some(fy as usize * self.w + fx as usize)
    }

    /// Whether a point of the board is copper (`None` outside the grid).
    pub fn at(&self, x: f64, y: f64) -> Option<bool> {
        self.index_at(x, y).map(|i| self.on[i])
    }

    /// Draws the area inside closed paths, even-odd: a path inside another one is a hole (a board with its cutouts).
    pub fn draw_even_odd(&mut self, paths: &[Vec<(f64, f64)>]) {
        let (mut x0, mut x1, mut y0, mut y1) = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for p in paths.iter().flatten() {
            x0 = x0.min(p.0);
            x1 = x1.max(p.0);
            y0 = y0.min(p.1);
            y1 = y1.max(p.1);
        }
        if x0 <= x1 && y0 <= y1 {
            self.fill(x0, x1, y0, y1, |px, py| {
                paths
                    .iter()
                    .filter(|p| p.len() >= 3)
                    .fold(false, |a, p| a ^ inside_polygon(px, py, p))
            });
        }
    }

    pub fn count(&self) -> usize {
        self.on.iter().filter(|b| **b).count()
    }

    fn fill(
        &mut self,
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        inside: impl Fn(f64, f64) -> bool,
    ) {
        let cell = |v: f64, o: f64| ((v - o) / self.res - 0.5).floor().max(0.0) as usize;
        let (ix0, ix1) = (
            cell(x_min, self.x0),
            (cell(x_max, self.x0) + 1).min(self.w.saturating_sub(1)),
        );
        let (iy0, iy1) = (
            cell(y_min, self.y0),
            (cell(y_max, self.y0) + 1).min(self.h.saturating_sub(1)),
        );
        for iy in iy0..=iy1 {
            for ix in ix0..=ix1 {
                let (x, y) = self.centre(ix as f64, iy as f64);
                if inside(x, y) {
                    self.on[iy * self.w + ix] = true;
                }
            }
        }
    }

    /// Like `fill`, for a convex shape: every row only looks at the cells between the ends of the
    /// shape on that row (`span` gives them for the height of the row), so a long thin diagonal
    /// track costs its length and width, not the area of its box.
    fn fill_spans(
        &mut self,
        y_min: f64,
        y_max: f64,
        span: impl Fn(f64) -> Option<(f64, f64)>,
        inside: impl Fn(f64, f64) -> bool,
    ) {
        let cell = |v: f64, o: f64| ((v - o) / self.res - 0.5).floor().max(0.0) as usize;
        let (iy0, iy1) = (
            cell(y_min, self.y0),
            (cell(y_max, self.y0) + 1).min(self.h.saturating_sub(1)),
        );
        for iy in iy0..=iy1 {
            let (_, y) = self.centre(0.0, iy as f64);
            let Some((lo, hi)) = span(y) else { continue };
            let ix0 = cell(lo, self.x0).saturating_sub(1);
            let ix1 = (cell(hi, self.x0) + 2).min(self.w.saturating_sub(1));
            for ix in ix0..=ix1 {
                let (x, y) = self.centre(ix as f64, iy as f64);
                if inside(x, y) {
                    self.on[iy * self.w + ix] = true;
                }
            }
        }
    }

    /// Draws the shapes of a scene.
    pub fn draw(&mut self, scene: &Scene) {
        for s in &scene.shapes {
            match s {
                Shape::Disc { x, y, r } => self.fill(x - r, x + r, y - r, y + r, |px, py| {
                    (px - x).powi(2) + (py - y).powi(2) <= r * r
                }),
                Shape::Rect { x, y, w, h } => self.fill(
                    x - w / 2.0,
                    x + w / 2.0,
                    y - h / 2.0,
                    y + h / 2.0,
                    |px, py| (px - x).abs() <= w / 2.0 && (py - y).abs() <= h / 2.0,
                ),
                Shape::Obround { x, y, w, h } => {
                    // a capsule along the longer side
                    let (r, half) = (w.min(*h) / 2.0, (w.max(*h) - w.min(*h)) / 2.0);
                    let (ax, ay, bx, by) = if w >= h {
                        (x - half, *y, x + half, *y)
                    } else {
                        (*x, y - half, *x, y + half)
                    };
                    self.fill_spans(
                        y - h / 2.0,
                        y + h / 2.0,
                        |row| capsule_span(row, (ax, ay), (bx, by), r),
                        |px, py| capsule_distance(px, py, ax, ay, bx, by) <= r,
                    )
                }
                Shape::Stroke { x0, y0, x1, y1, w } => {
                    let r = w / 2.0;
                    self.fill_spans(
                        y0.min(*y1) - r,
                        y0.max(*y1) + r,
                        |row| capsule_span(row, (*x0, *y0), (*x1, *y1), r),
                        |px, py| capsule_distance(px, py, *x0, *y0, *x1, *y1) <= r,
                    )
                }
                // a region with fewer than three points has no inside
                Shape::Region { pts } if pts.len() < 3 => {}
                Shape::Region { pts } => {
                    let (min_x, max_x) = (
                        pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
                        pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
                    );
                    let (min_y, max_y) = (
                        pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
                        pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
                    );
                    self.fill(min_x, max_x, min_y, max_y, |px, py| {
                        inside_polygon(px, py, pts)
                    });
                }
            }
        }
    }
}

/// The ends, on the row at height `y`, of the capsule that is every point within `r` of the segment from `a` to `b`; `None` when the row does not meet it.
pub fn capsule_span(y: f64, a: (f64, f64), b: (f64, f64), r: f64) -> Option<(f64, f64)> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    // the two round ends
    for c in [a, b] {
        let dy = y - c.1;
        if dy.abs() <= r {
            let s = (r * r - dy * dy).max(0.0).sqrt();
            lo = lo.min(c.0 - s);
            hi = hi.max(c.0 + s);
        }
    }
    // the strip between them
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx.hypot(dy);
    if len > 0.0 {
        let (nx, ny) = (-dy / len * r, dx / len * r);
        let c = [
            (a.0 + nx, a.1 + ny),
            (b.0 + nx, b.1 + ny),
            (b.0 - nx, b.1 - ny),
            (a.0 - nx, a.1 - ny),
        ];
        for i in 0..4 {
            let (p, q) = (c[i], c[(i + 1) % 4]);
            if (p.1 <= y && y <= q.1) || (q.1 <= y && y <= p.1) {
                if p.1 == q.1 {
                    lo = lo.min(p.0).min(q.0);
                    hi = hi.max(p.0).max(q.0);
                } else {
                    let x = p.0 + (y - p.1) * (q.0 - p.0) / (q.1 - p.1);
                    lo = lo.min(x);
                    hi = hi.max(x);
                }
            }
        }
    }
    (lo <= hi).then_some((lo, hi))
}

pub fn capsule_distance(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let (dx, dy) = (bx - ax, by - ay);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0)
    };
    ((px - (ax + t * dx)).powi(2) + (py - (ay + t * dy)).powi(2)).sqrt()
}

/// Even-odd test of a point against a polygon.
pub fn inside_polygon(px: f64, py: f64, pts: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = pts.len() - 1;
    for i in 0..pts.len() {
        let (xi, yi, xj, yj) = (pts[i].0, pts[i].1, pts[j].0, pts[j].1);
        if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Every number of every shape must be finite: a broken layer is refused, not drawn.
pub fn check_finite(scene: &Scene) -> Result<(), String> {
    for (i, s) in scene.shapes.iter().enumerate() {
        let ok = match s {
            Shape::Disc { x, y, r } => [x, y, r].iter().all(|v| v.is_finite()),
            Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => {
                [x, y, w, h].iter().all(|v| v.is_finite())
            }
            Shape::Stroke { x0, y0, x1, y1, w } => {
                [x0, y0, x1, y1, w].iter().all(|v| v.is_finite())
            }
            Shape::Region { pts } => pts.iter().all(|p| p.0.is_finite() && p.1.is_finite()),
        };
        if !ok {
            return Err(format!("shape {}: a coordinate is not a number", i + 1));
        }
    }
    Ok(())
}

/// The copper of a scene on a grid with a margin around it.
pub fn from_scene(scene: &Scene, margin_mm: f64, res: f64) -> Result<Bitmap, String> {
    check_finite(scene)?;
    let mut b = Bitmap::new(&scene.bounds(), margin_mm, res)?;
    b.draw(scene);
    Ok(b)
}
