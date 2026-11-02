//! The shapes of a layer, in millimetres, with y up as in the file.

use eh3d_parser_excellon::ExcellonFile;
use eh3d_parser_gerber::{Aperture, GerberFile, Operation, Point, Polarity, QuadrantMode, Units};

/// The segments an arc is cut in when it is drawn.
const ARC_STEPS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Bounds {
    pub fn empty() -> Bounds {
        Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }
    pub fn include(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
    }
    pub fn union(&self, other: &Bounds) -> Bounds {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Bounds {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }
    pub fn width(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max_x - self.min_x
        }
    }
    pub fn height(&self) -> f64 {
        if self.is_empty() {
            0.0
        } else {
            self.max_y - self.min_y
        }
    }
    pub fn center(&self) -> (f64, f64) {
        (
            (self.min_x + self.max_x) / 2.0,
            (self.min_y + self.max_y) / 2.0,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Disc {
        x: f64,
        y: f64,
        r: f64,
    },
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    Obround {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    /// A track: a line drawn with a round pen of width `w`.
    Stroke {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        w: f64,
    },
    Region {
        pts: Vec<(f64, f64)>,
    },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    pub shapes: Vec<Shape>,
    /// What was not drawn, once each, with the reason.
    pub notes: Vec<String>,
}

impl Scene {
    pub fn bounds(&self) -> Bounds {
        let mut b = Bounds::empty();
        for s in &self.shapes {
            match s {
                Shape::Disc { x, y, r } => {
                    b.include(x - r, y - r);
                    b.include(x + r, y + r);
                }
                Shape::Rect { x, y, w, h } | Shape::Obround { x, y, w, h } => {
                    b.include(x - w / 2.0, y - h / 2.0);
                    b.include(x + w / 2.0, y + h / 2.0);
                }
                Shape::Stroke { x0, y0, x1, y1, w } => {
                    for (x, y) in [(x0, y0), (x1, y1)] {
                        b.include(x - w / 2.0, y - w / 2.0);
                        b.include(x + w / 2.0, y + w / 2.0);
                    }
                }
                Shape::Region { pts } => pts.iter().for_each(|(x, y)| b.include(*x, *y)),
            }
        }
        b
    }

    fn note(&mut self, text: &str) {
        if !self.notes.iter().any(|n| n == text) {
            self.notes.push(text.to_string());
        }
    }
}

/// The outline of a rectangle of `w` x `h` (sides parallel to the axes) dragged from `a` to `b`: the Minkowski sum of the segment and the rectangle, a convex polygon of up to six corners.
pub fn swept_rectangle(a: (f64, f64), b: (f64, f64), w: f64, h: f64) -> Vec<(f64, f64)> {
    let (hx, hy) = (w / 2.0, h / 2.0);
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(8);
    for c in [a, b] {
        for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            pts.push((c.0 + sx * hx, c.1 + sy * hy));
        }
    }
    convex_hull(pts)
}

/// The convex hull, counter-clockwise, of a few points (Andrew's monotone chain).
fn convex_hull(mut pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    pts.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
    pts.dedup_by(|p, q| (p.0 - q.0).abs() < 1e-12 && (p.1 - q.1).abs() < 1e-12);
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<(f64, f64)> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let order: Vec<(f64, f64)> = if pass == 0 {
            pts.clone()
        } else {
            pts.iter().rev().cloned().collect()
        };
        for q in order {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], q) <= 1e-12
            {
                hull.pop();
            }
            hull.push(q);
        }
        hull.pop();
    }
    hull
}

/// The points of an arc from `from` to `to` around the centre given as an offset from `from`.
pub fn arc_points(
    from: (f64, f64),
    to: (f64, f64),
    offset: (f64, f64),
    clockwise: bool,
    quadrant: QuadrantMode,
) -> Vec<(f64, f64)> {
    let candidates: Vec<(f64, f64)> = match quadrant {
        QuadrantMode::Multi => vec![(from.0 + offset.0, from.1 + offset.1)],
        // in single quadrant mode the offsets have no sign: the centre is the one of the four that fits
        QuadrantMode::Single => [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)]
            .iter()
            .map(|(sx, sy)| (from.0 + sx * offset.0.abs(), from.1 + sy * offset.1.abs()))
            .collect(),
    };
    let best = candidates
        .into_iter()
        .min_by(|a, b| {
            mismatch(*a, from, to)
                .partial_cmp(&mismatch(*b, from, to))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(from);
    let r = ((from.0 - best.0).powi(2) + (from.1 - best.1).powi(2)).sqrt();
    if r < 1e-9 {
        return vec![from, to];
    }
    let a0 = (from.1 - best.1).atan2(from.0 - best.0);
    let mut a1 = (to.1 - best.1).atan2(to.0 - best.0);
    if clockwise {
        while a1 >= a0 {
            a1 -= 2.0 * std::f64::consts::PI;
        }
    } else {
        while a1 <= a0 {
            a1 += 2.0 * std::f64::consts::PI;
        }
    }
    (0..=ARC_STEPS)
        .map(|i| a0 + (a1 - a0) * i as f64 / ARC_STEPS as f64)
        .map(|a| (best.0 + r * a.cos(), best.1 + r * a.sin()))
        .collect()
}

fn mismatch(c: (f64, f64), from: (f64, f64), to: (f64, f64)) -> f64 {
    let d = |p: (f64, f64)| ((p.0 - c.0).powi(2) + (p.1 - c.1).powi(2)).sqrt();
    (d(from) - d(to)).abs()
}

/// The shapes of a Gerber layer. Dark drawing only: clear polarity, step and repeat, macro apertures and regions made of arcs are noted, not drawn.
pub fn from_gerber(file: &GerberFile) -> Scene {
    let k = file.units.unwrap_or(Units::Millimetres).to_mm(1.0);
    let mut scene = Scene::default();
    let mut aperture: Option<&Aperture> = None;
    let mut region: Option<Vec<(f64, f64)>> = None;
    let p = |pt: &Point| (pt.x * k, pt.y * k);
    for c in &file.commands {
        match &c.op {
            Operation::SelectAperture(n) => aperture = file.apertures.get(n),
            Operation::Polarity(Polarity::Clear) => scene.note("clear polarity is drawn as dark"),
            Operation::StepRepeat(Some(_)) => scene.note("step and repeat is not drawn"),
            Operation::RegionStart => region = Some(Vec::new()),
            Operation::RegionEnd => {
                if let Some(pts) = region.take() {
                    if pts.len() >= 3 {
                        scene.shapes.push(Shape::Region { pts });
                    }
                }
            }
            Operation::Move { to } => {
                if let Some(pts) = region.as_mut() {
                    if pts.len() >= 3 {
                        scene.shapes.push(Shape::Region {
                            pts: std::mem::take(pts),
                        });
                    }
                    pts.clear();
                    pts.push(p(to));
                }
            }
            Operation::Flash { at } => {
                let (x, y) = p(at);
                match aperture {
                    Some(Aperture::Circle { diameter, .. }) => scene.shapes.push(Shape::Disc {
                        x,
                        y,
                        r: diameter * k / 2.0,
                    }),
                    Some(Aperture::Rectangle { x: w, y: h, .. }) => {
                        scene.shapes.push(Shape::Rect {
                            x,
                            y,
                            w: w * k,
                            h: h * k,
                        })
                    }
                    Some(Aperture::Obround { x: w, y: h, .. }) => {
                        scene.shapes.push(Shape::Obround {
                            x,
                            y,
                            w: w * k,
                            h: h * k,
                        })
                    }
                    Some(Aperture::Polygon { diameter, .. }) => {
                        scene.note("a polygon aperture is drawn as a circle");
                        scene.shapes.push(Shape::Disc {
                            x,
                            y,
                            r: diameter * k / 2.0,
                        });
                    }
                    Some(Aperture::Macro { .. }) => scene.note("a macro aperture is not drawn"),
                    None => scene.note("a flash with no aperture selected is not drawn"),
                }
            }
            Operation::Line { from, to } => {
                if let Some(pts) = region.as_mut() {
                    if pts.is_empty() {
                        pts.push(p(from));
                    }
                    pts.push(p(to));
                } else {
                    let ((x0, y0), (x1, y1)) = (p(from), p(to));
                    let w = match aperture {
                        Some(Aperture::Circle { diameter, .. }) => diameter * k,
                        Some(Aperture::Rectangle { x, y, .. }) => {
                            scene.shapes.push(Shape::Region {
                                pts: swept_rectangle((x0, y0), (x1, y1), x * k, y * k),
                            });
                            continue;
                        }
                        Some(Aperture::Obround { x, y, .. }) => {
                            scene.note("a track with an obround pen is drawn with the rectangle that holds the pen");
                            scene.shapes.push(Shape::Region {
                                pts: swept_rectangle((x0, y0), (x1, y1), x * k, y * k),
                            });
                            continue;
                        }
                        _ => {
                            scene.note("a track with no usable aperture is not drawn");
                            continue;
                        }
                    };
                    scene.shapes.push(Shape::Stroke { x0, y0, x1, y1, w });
                }
            }
            Operation::Arc {
                from,
                to,
                center_offset,
                clockwise,
                quadrant,
            } => {
                let pts = arc_points(
                    p(from),
                    p(to),
                    (center_offset.0 * k, center_offset.1 * k),
                    *clockwise,
                    *quadrant,
                );
                if let Some(r) = region.as_mut() {
                    if r.is_empty() {
                        r.push(pts[0]);
                    }
                    r.extend(pts.iter().skip(1));
                } else if let Some(Aperture::Circle { diameter, .. }) = aperture {
                    for pair in pts.windows(2) {
                        scene.shapes.push(Shape::Stroke {
                            x0: pair[0].0,
                            y0: pair[0].1,
                            x1: pair[1].0,
                            y1: pair[1].1,
                            w: diameter * k,
                        });
                    }
                } else {
                    scene.note("an arc with no round pen is not drawn");
                }
            }
            Operation::Polarity(_) | Operation::StepRepeat(None) => {}
        }
    }
    scene
}

/// The holes of a drill file as discs of the diameter of their tool (and slots as tracks).
pub fn from_excellon(file: &ExcellonFile) -> Scene {
    let k = file.units.map(|u| u.to_mm(1.0)).unwrap_or(1.0);
    let mut scene = Scene::default();
    let diameter = |tool: u32| file.tools.get(&tool).map(|t| t.diameter * k).unwrap_or(0.0);
    for h in &file.drills {
        scene.shapes.push(Shape::Disc {
            x: h.at.x * k,
            y: h.at.y * k,
            r: diameter(h.tool) / 2.0,
        });
    }
    for s in &file.slots {
        scene.shapes.push(Shape::Stroke {
            x0: s.from.x * k,
            y0: s.from.y * k,
            x1: s.to.x * k,
            y1: s.to.y * k,
            w: diameter(s.tool),
        });
    }
    if !file.routes.is_empty() {
        scene.note("routes of the drill file are not drawn");
    }
    scene
}
