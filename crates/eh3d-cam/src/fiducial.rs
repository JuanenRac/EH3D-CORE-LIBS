//! Scaling of the work and compensation for a board that is stretched, turned or deformed: pairs of reference points (where a mark should be and where the camera or the
//! probe measured it) give a similarity (two pairs: scale, turn and shift) or an affine transformation (three or more pairs, least squares), applied to every path and hole.

use crate::hole::Hole;
use crate::isolate::Pass;

type Pt = (f64, f64);

/// `x' = a x + b y + e`, `y' = c x + d y + f`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine {
    pub fn identity() -> Affine {
        Affine::scale(1.0, 1.0)
    }

    pub fn scale(sx: f64, sy: f64) -> Affine {
        Affine {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn is_identity(&self) -> bool {
        *self == Affine::identity()
    }

    pub fn apply(&self, p: Pt) -> Pt {
        (
            self.a * p.0 + self.b * p.1 + self.e,
            self.c * p.0 + self.d * p.1 + self.f,
        )
    }

    pub fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// First `self`, then `next`.
    pub fn then(&self, next: &Affine) -> Affine {
        let (e, f) = next.apply((self.e, self.f));
        Affine {
            a: next.a * self.a + next.b * self.c,
            b: next.a * self.b + next.b * self.d,
            c: next.c * self.a + next.d * self.c,
            d: next.c * self.b + next.d * self.d,
            e,
            f,
        }
    }

    /// The transformation must be finite and must not turn the board over or collapse it.
    pub fn validate(&self) -> Result<(), String> {
        let v = [self.a, self.b, self.c, self.d, self.e, self.f];
        if v.iter().any(|x| !x.is_finite()) {
            return Err("the transformation is not a finite number".into());
        }
        if self.det() <= 1e-9 {
            return Err(format!(
                "the transformation has a determinant of {}: it turns the board over or collapses it",
                self.det()
            ));
        }
        Ok(())
    }
}

/// Where a mark should be (in the design) and where it was measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pair {
    pub design: Pt,
    pub measured: Pt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    /// Two pairs give a similarity, three or more an affine transformation.
    Auto,
    Similarity,
    Affine,
}

impl Model {
    pub fn parse(v: &str) -> Result<Model, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Model::Auto),
            "similarity" => Ok(Model::Similarity),
            "affine" => Ok(Model::Affine),
            other => Err(format!(
                "fiducial_model '{other}': it must be auto, similarity or affine"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Model::Auto => "auto",
            Model::Similarity => "similarity",
            Model::Affine => "affine",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// How far the scale of each axis may be from 1 (0.05 is 5 %).
    pub max_scale_deviation: f64,
    /// The farthest a measured mark may be from where the fit puts it, in mm.
    pub max_residual_mm: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_scale_deviation: 0.05,
            max_residual_mm: 0.2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fit {
    pub affine: Affine,
    /// For each pair, how far the fit puts the design mark from the measured one, in mm.
    pub residuals_mm: Vec<f64>,
    pub scale_x: f64,
    pub scale_y: f64,
    pub rotation_deg: f64,
}

/// Reads `design_x, design_y, measured_x, measured_y` (commas or spaces).
pub fn parse_pair(text: &str, bare: eh3d_raster::units::Unit) -> Result<Pair, String> {
    let parts: Vec<&str> = text
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() != 4 {
        return Err(format!(
            "fiducial '{text}': it needs four numbers, design x, design y, measured x, measured y"
        ));
    }
    let mut v = [0.0; 4];
    for (slot, part) in v.iter_mut().zip(&parts) {
        *slot =
            eh3d_raster::units::parse_length(part, bare).map_err(|e| format!("fiducial: {e}"))?;
    }
    Ok(Pair {
        design: (v[0], v[1]),
        measured: (v[2], v[3]),
    })
}

fn centroid(pts: impl Iterator<Item = Pt> + Clone) -> Pt {
    let n = pts.clone().count() as f64;
    let (sx, sy) = pts.fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    (sx / n, sy / n)
}

/// Fits the transformation that takes the design marks to the measured ones.
pub fn fit(pairs: &[Pair], model: Model, limits: &Limits) -> Result<Fit, String> {
    for (i, p) in pairs.iter().enumerate() {
        let v = [p.design.0, p.design.1, p.measured.0, p.measured.1];
        if v.iter().any(|x| !x.is_finite()) {
            return Err(format!("fiducial {}: a coordinate is not a number", i + 1));
        }
    }
    if pairs.len() < 2 {
        return Err(format!(
            "{} reference points: at least 2 are needed",
            pairs.len()
        ));
    }
    let model = match model {
        Model::Auto if pairs.len() >= 3 => Model::Affine,
        Model::Auto => Model::Similarity,
        m => m,
    };
    if model == Model::Affine && pairs.len() < 3 {
        return Err(format!(
            "{} reference points: an affine fit needs at least 3",
            pairs.len()
        ));
    }
    let cp = centroid(pairs.iter().map(|p| p.design));
    let cq = centroid(pairs.iter().map(|p| p.measured));
    let dp: Vec<Pt> = pairs
        .iter()
        .map(|p| (p.design.0 - cp.0, p.design.1 - cp.1))
        .collect();
    let dq: Vec<Pt> = pairs
        .iter()
        .map(|p| (p.measured.0 - cq.0, p.measured.1 - cq.1))
        .collect();
    let (sxx, syy, sxy): (f64, f64, f64) = dp.iter().fold((0.0, 0.0, 0.0), |s, p| {
        (s.0 + p.0 * p.0, s.1 + p.1 * p.1, s.2 + p.0 * p.1)
    });
    if sxx + syy < 1e-9 {
        return Err("the reference points are all in the same place".into());
    }
    let (a, b, c, d) = if model == Model::Similarity {
        // complex numbers: (a + ib) = sum(conj(p) q) / sum(|p|^2)
        let (re, im) = dp.iter().zip(&dq).fold((0.0, 0.0), |s, (p, q)| {
            (s.0 + p.0 * q.0 + p.1 * q.1, s.1 + p.0 * q.1 - p.1 * q.0)
        });
        let k = sxx + syy;
        (re / k, -im / k, im / k, re / k)
    } else {
        let det = sxx * syy - sxy * sxy;
        if det.abs() < 1e-9 * (sxx + syy).powi(2) {
            return Err(
                "the reference points lie on a line: an affine fit needs three that do not".into(),
            );
        }
        let (bx, by): (Pt, Pt) = dp
            .iter()
            .zip(&dq)
            .fold(((0.0, 0.0), (0.0, 0.0)), |s, (p, q)| {
                (
                    (s.0 .0 + p.0 * q.0, s.0 .1 + p.1 * q.0),
                    (s.1 .0 + p.0 * q.1, s.1 .1 + p.1 * q.1),
                )
            });
        // [a b] = [bx.0 bx.1] * inverse([[sxx, sxy], [sxy, syy]])
        (
            (bx.0 * syy - bx.1 * sxy) / det,
            (bx.1 * sxx - bx.0 * sxy) / det,
            (by.0 * syy - by.1 * sxy) / det,
            (by.1 * sxx - by.0 * sxy) / det,
        )
    };
    let affine = Affine {
        a,
        b,
        c,
        d,
        e: cq.0 - (a * cp.0 + b * cp.1),
        f: cq.1 - (c * cp.0 + d * cp.1),
    };
    affine.validate()?;
    let (scale_x, scale_y) = (a.hypot(c), b.hypot(d));
    for (name, s) in [("x", scale_x), ("y", scale_y)] {
        if (s - 1.0).abs() > limits.max_scale_deviation {
            return Err(format!(
                "the scale of {name} is {s:.4}: more than {} away from 1 (the marks were probably mismatched; fiducial_max_scale_deviation allows more)",
                limits.max_scale_deviation
            ));
        }
    }
    let residuals_mm: Vec<f64> = pairs
        .iter()
        .map(|p| {
            let q = affine.apply(p.design);
            (q.0 - p.measured.0).hypot(q.1 - p.measured.1)
        })
        .collect();
    if let Some((i, r)) = residuals_mm
        .iter()
        .enumerate()
        .find(|(_, r)| **r > limits.max_residual_mm)
    {
        return Err(format!(
            "fiducial {} is {r:.3} mm from where the fit puts it: more than {} mm (a wrong measurement, or a deformation the model cannot follow; fiducial_max_residual_mm allows more)",
            i + 1,
            limits.max_residual_mm
        ));
    }
    Ok(Fit {
        affine,
        residuals_mm,
        scale_x,
        scale_y,
        rotation_deg: c.atan2(a).to_degrees(),
    })
}

/// Moves every point of the passes. A path that is no longer finite is an error.
pub fn apply_to_passes(passes: &mut [Pass], t: &Affine) -> Result<(), String> {
    t.validate()?;
    for (k, pass) in passes.iter_mut().enumerate() {
        for path in pass.paths.iter_mut().chain(pass.open.iter_mut()) {
            for p in path.iter_mut() {
                *p = t.apply(*p);
                if !(p.0.is_finite() && p.1.is_finite()) {
                    return Err(format!("pass {}: a point is no longer a number", k + 1));
                }
            }
        }
        pass.offset_mm *= (t.a.hypot(t.c) + t.b.hypot(t.d)) / 2.0;
    }
    Ok(())
}

/// Moves the centres of the holes (their diameters stay).
pub fn apply_to_holes(holes: &mut [Hole], t: &Affine) -> Result<(), String> {
    t.validate()?;
    for (i, h) in holes.iter_mut().enumerate() {
        let p = t.apply((h.x_mm, h.y_mm));
        if !(p.0.is_finite() && p.1.is_finite()) {
            return Err(format!(
                "hole {}: the position is no longer a number",
                i + 1
            ));
        }
        h.x_mm = p.0;
        h.y_mm = p.1;
    }
    Ok(())
}
