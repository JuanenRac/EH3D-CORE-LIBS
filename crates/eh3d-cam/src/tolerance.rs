//! Where the conflict tolerance applies: to every gap between two pieces of copper, or only to the gaps next to a pad (a pad can take a little scraping, a track should not).

use crate::bitmap::Bitmap;
use crate::isolate::{close_pieces, Closeness};
use eh3d_geometry::scene::{Scene, Shape};
use eh3d_raster::edt::distance_mm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToleranceScope {
    /// Every gap (the default).
    #[default]
    All,
    /// Only a gap that has a pad beside it.
    Pads,
}

impl ToleranceScope {
    pub fn parse(v: &str) -> Result<ToleranceScope, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "all" | "everything" | "all_objects" => Ok(ToleranceScope::All),
            "pads" | "pad" | "only_pads" => Ok(ToleranceScope::Pads),
            other => Err(format!(
                "conflict_tolerance_scope '{other}': it must be all or pads"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ToleranceScope::All => "all",
            ToleranceScope::Pads => "pads",
        }
    }
}

/// Is this shape a pad (a flash: a disc, a rectangle or an obround)?
pub fn is_pad(s: &Shape) -> bool {
    matches!(
        s,
        Shape::Disc { .. } | Shape::Rect { .. } | Shape::Obround { .. }
    )
}

/// The places where the finest tool does not fit between pieces of copper once the tolerance is allowed. With `ToleranceScope::All` that is `close_pieces` with the tool less the
/// tolerance; with `Pads` a gap narrower than the tool but wide enough for the tool less the tolerance is accepted only when a pad is beside it, and every other gap is judged
/// against the whole tool.
pub fn close_pieces_scoped(
    scene: &Scene,
    finest_mm: f64,
    tolerance_mm: f64,
    scope: ToleranceScope,
    res_mm: f64,
) -> Result<Vec<Closeness>, String> {
    let lenient_gap = (finest_mm - tolerance_mm).max(0.02);
    if scope == ToleranceScope::All || tolerance_mm <= 0.0 {
        return close_pieces(scene, lenient_gap, res_mm);
    }
    let strict = close_pieces(scene, finest_mm, res_mm)?;
    if strict.is_empty() {
        return Ok(strict);
    }
    let lenient = close_pieces(scene, lenient_gap, res_mm)?;
    let pads_only = Scene {
        shapes: scene.shapes.iter().filter(|s| is_pad(s)).cloned().collect(),
        notes: Vec::new(),
    };
    if pads_only.shapes.is_empty() {
        return Ok(strict);
    }
    let mut bm = Bitmap::new(&scene.bounds(), finest_mm + 4.0 * res_mm, res_mm)?;
    bm.draw(&pads_only);
    let dist = distance_mm(&bm.on, bm.w, bm.h, bm.res);
    let beside_pad = |c: &Closeness| {
        bm.index_at(c.x, c.y)
            .is_some_and(|i| f64::from(dist[i]) <= finest_mm / 2.0 + 2.0 * res_mm)
    };
    Ok(strict
        .into_iter()
        .filter(|c| {
            let still = lenient
                .iter()
                .any(|l| (l.x - c.x).hypot(l.y - c.y) <= finest_mm);
            still || !beside_pad(c)
        })
        .collect())
}
