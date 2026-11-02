//! The solder mask: the openings, as a Gerber layer. They come from the mask layer of the design (each opening grown by a margin) or, when there is none, from the pads of the
//! copper (flashed circles, rectangles and obrounds; tracks stay covered), each grown by the margin. Silkscreen is not produced.

use eh3d_geometry::scene::{Scene, Shape};
use std::collections::BTreeMap;

fn finite(s: &Shape) -> bool {
    match s {
        Shape::Disc { x, y, r } => [x, y, r].iter().all(|v| v.is_finite()),
        Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => {
            [x, y, w, h].iter().all(|v| v.is_finite())
        }
        Shape::Stroke { x0, y0, x1, y1, w } => [x0, y0, x1, y1, w].iter().all(|v| v.is_finite()),
        Shape::Region { pts } => pts.iter().all(|p| p.0.is_finite() && p.1.is_finite()),
    }
}

/// The opening of one shape grown by `m` (negative shrinks); `None` when nothing is left of it.
fn grow(s: &Shape, m: f64) -> Option<Shape> {
    match s {
        Shape::Disc { x, y, r } => (r + m > 0.0).then(|| Shape::Disc {
            x: *x,
            y: *y,
            r: r + m,
        }),
        Shape::Rect { x, y, w, h } => {
            (w + 2.0 * m > 0.0 && h + 2.0 * m > 0.0).then(|| Shape::Rect {
                x: *x,
                y: *y,
                w: w + 2.0 * m,
                h: h + 2.0 * m,
            })
        }
        Shape::Obround { x, y, w, h } => {
            (w + 2.0 * m > 0.0 && h + 2.0 * m > 0.0).then(|| Shape::Obround {
                x: *x,
                y: *y,
                w: w + 2.0 * m,
                h: h + 2.0 * m,
            })
        }
        Shape::Stroke { x0, y0, x1, y1, w } => (w + 2.0 * m > 0.0).then(|| Shape::Stroke {
            x0: *x0,
            y0: *y0,
            x1: *x1,
            y1: *y1,
            w: w + 2.0 * m,
        }),
        Shape::Region { pts } => Some(Shape::Region { pts: pts.clone() }),
    }
}

/// The openings of the mask. With `mask` they are its shapes grown by `margin_mm`; without, the pads of `copper` grown by it.
pub fn openings(copper: &Scene, mask: Option<&Scene>, margin_mm: f64) -> Result<Scene, String> {
    if !(margin_mm.is_finite() && (-1.0..=1.0).contains(&margin_mm)) {
        return Err(format!(
            "mask_margin_mm is {margin_mm}: it must be from -1 to 1"
        ));
    }
    let (source, from_mask) = match mask {
        Some(m) => (m, true),
        None => (copper, false),
    };
    let mut out = Scene::default();
    let (mut skipped_tracks, mut vanished, mut regions) = (0usize, 0usize, 0usize);
    for (i, s) in source.shapes.iter().enumerate() {
        if !finite(s) {
            return Err(format!(
                "{} shape {}: a coordinate is not a number",
                if from_mask { "mask" } else { "copper" },
                i + 1
            ));
        }
        if !from_mask && matches!(s, Shape::Stroke { .. } | Shape::Region { .. }) {
            skipped_tracks += 1;
            continue;
        }
        match grow(s, margin_mm) {
            Some(Shape::Region { pts }) => {
                if margin_mm > 0.0 && pts.len() >= 3 {
                    // the margin round a region: a round pen along its edge, drawn with the region
                    let n = pts.len();
                    for k in 0..n {
                        let (a, b) = (pts[k], pts[(k + 1) % n]);
                        out.shapes.push(Shape::Stroke {
                            x0: a.0,
                            y0: a.1,
                            x1: b.0,
                            y1: b.1,
                            w: 2.0 * margin_mm,
                        });
                    }
                } else if margin_mm < 0.0 {
                    regions += 1;
                }
                out.shapes.push(Shape::Region { pts });
            }
            Some(g) => out.shapes.push(g),
            None => vanished += 1,
        }
    }
    if out.shapes.is_empty() {
        return Err(if from_mask {
            "the mask layer has no openings".into()
        } else {
            "the copper has no pads to open".to_string()
        });
    }
    if skipped_tracks > 0 {
        out.notes.push(format!(
            "{skipped_tracks} tracks and regions of copper stay covered by the mask"
        ));
    }
    if vanished > 0 {
        out.notes.push(format!(
            "{vanished} openings are gone: the negative margin is larger than they are"
        ));
    }
    if regions > 0 {
        out.notes.push(format!(
            "{regions} regions keep their size: a negative margin does not shrink a region"
        ));
    }
    Ok(out)
}

fn mm6(v: f64) -> Result<i64, String> {
    let n = (v * 1e6).round();
    if n.abs() > 9.0e12 {
        return Err(format!("{v} mm is too far from the origin to write"));
    }
    Ok(n as i64)
}

/// The scene as a Gerber layer (millimetres, 4.6): every opening drawn dark.
pub fn write_gerber(scene: &Scene) -> Result<String, String> {
    let mut apertures: BTreeMap<String, usize> = BTreeMap::new();
    let mut body = String::new();
    fn aperture(def: String, apertures: &mut BTreeMap<String, usize>) -> usize {
        let next = apertures.len() + 10;
        *apertures.entry(def).or_insert(next)
    }
    let mut current = 0usize;
    let size = |v: f64| format!("{:.6}", v);
    for (i, s) in scene.shapes.iter().enumerate() {
        if !finite(s) {
            return Err(format!("shape {}: a coordinate is not a number", i + 1));
        }
        let select = |code: usize, current: &mut usize, body: &mut String| {
            if *current != code {
                body.push_str(&format!("D{code}*\n"));
                *current = code;
            }
        };
        match s {
            Shape::Disc { x, y, r } => {
                let code = aperture(format!("C,{}", size(2.0 * r)), &mut apertures);
                select(code, &mut current, &mut body);
                body.push_str(&format!("X{}Y{}D03*\n", mm6(*x)?, mm6(*y)?));
            }
            Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => {
                let kind = if matches!(s, Shape::Rect { .. }) {
                    "R"
                } else {
                    "O"
                };
                let code = aperture(format!("{kind},{}X{}", size(*w), size(*h)), &mut apertures);
                select(code, &mut current, &mut body);
                body.push_str(&format!("X{}Y{}D03*\n", mm6(*x)?, mm6(*y)?));
            }
            Shape::Stroke { x0, y0, x1, y1, w } => {
                let code = aperture(format!("C,{}", size(*w)), &mut apertures);
                select(code, &mut current, &mut body);
                body.push_str(&format!(
                    "X{}Y{}D02*\nX{}Y{}D01*\n",
                    mm6(*x0)?,
                    mm6(*y0)?,
                    mm6(*x1)?,
                    mm6(*y1)?
                ));
            }
            Shape::Region { pts } => {
                if pts.len() < 3 {
                    return Err(format!(
                        "shape {}: a region has {} points, it needs at least 3",
                        i + 1,
                        pts.len()
                    ));
                }
                body.push_str("G36*\n");
                for (k, p) in pts.iter().chain(std::iter::once(&pts[0])).enumerate() {
                    body.push_str(&format!(
                        "X{}Y{}D0{}*\n",
                        mm6(p.0)?,
                        mm6(p.1)?,
                        if k == 0 { 2 } else { 1 }
                    ));
                }
                body.push_str("G37*\n");
            }
        }
    }
    let mut head = String::from("%FSLAX46Y46*%\n%MOMM*%\n%LPD*%\n");
    let mut list: Vec<(&usize, &String)> = apertures.iter().map(|(d, c)| (c, d)).collect();
    list.sort();
    for (code, def) in list {
        head.push_str(&format!("%ADD{code}{def}*%\n"));
    }
    Ok(format!("{head}G01*\n{body}M02*\n"))
}
