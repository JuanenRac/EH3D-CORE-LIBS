//! Moving the layer about before it is milled: turning it over for the other side of a double-sided board, turning it by quarter turns, repeating it on a panel, and the holes
//! that hold the board in the same place when it is turned over.

use crate::hole::Hole;
use eh3d_geometry::scene::{Bounds, Scene, Shape};

/// The most boards of a panel.
pub const MAX_PANEL: usize = 100;

/// The line the board is turned over on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// A vertical line: left and right change places.
    Vertical,
    /// A horizontal line: top and bottom change places.
    Horizontal,
}

impl Axis {
    /// `none` is no mirror at all.
    pub fn parse_option(v: &str) -> Result<Option<Axis>, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "none" | "no" | "off" | "" => Ok(None),
            "vertical" => Ok(Some(Axis::Vertical)),
            "horizontal" => Ok(Some(Axis::Horizontal)),
            other => Err(format!(
                "mirror '{other}': it must be none, vertical or horizontal"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Axis::Vertical => "vertical",
            Axis::Horizontal => "horizontal",
        }
    }

    /// Where the line is for a board with these bounds (through its middle).
    pub fn middle(self, b: &Bounds) -> f64 {
        match self {
            Axis::Vertical => (b.min_x + b.max_x) / 2.0,
            Axis::Horizontal => (b.min_y + b.max_y) / 2.0,
        }
    }

    fn flip(self, at: f64, p: (f64, f64)) -> (f64, f64) {
        match self {
            Axis::Vertical => (2.0 * at - p.0, p.1),
            Axis::Horizontal => (p.0, 2.0 * at - p.1),
        }
    }
}

fn map_shapes(scene: &Scene, f: impl Fn(&Shape) -> Shape) -> Scene {
    Scene {
        shapes: scene.shapes.iter().map(f).collect(),
        notes: scene.notes.clone(),
    }
}

/// The scene turned over on the line `at` (an X for a vertical line, a Y for a horizontal one).
pub fn mirror_scene(scene: &Scene, axis: Axis, at: f64) -> Scene {
    map_shapes(scene, |s| match s {
        Shape::Disc { x, y, r } => {
            let p = axis.flip(at, (*x, *y));
            Shape::Disc {
                x: p.0,
                y: p.1,
                r: *r,
            }
        }
        Shape::Rect { x, y, w, h } => {
            let p = axis.flip(at, (*x, *y));
            Shape::Rect {
                x: p.0,
                y: p.1,
                w: *w,
                h: *h,
            }
        }
        Shape::Obround { x, y, w, h } => {
            let p = axis.flip(at, (*x, *y));
            Shape::Obround {
                x: p.0,
                y: p.1,
                w: *w,
                h: *h,
            }
        }
        Shape::Stroke { x0, y0, x1, y1, w } => {
            let (a, b) = (axis.flip(at, (*x0, *y0)), axis.flip(at, (*x1, *y1)));
            Shape::Stroke {
                x0: a.0,
                y0: a.1,
                x1: b.0,
                y1: b.1,
                w: *w,
            }
        }
        Shape::Region { pts } => Shape::Region {
            pts: pts.iter().map(|p| axis.flip(at, *p)).collect(),
        },
    })
}

pub fn mirror_holes(holes: &[Hole], axis: Axis, at: f64) -> Vec<Hole> {
    holes
        .iter()
        .map(|h| {
            let p = axis.flip(at, (h.x_mm, h.y_mm));
            Hole {
                x_mm: p.0,
                y_mm: p.1,
                ..*h
            }
        })
        .collect()
}

fn turn(p: (f64, f64), c: (f64, f64), quarters: u8) -> (f64, f64) {
    let (dx, dy) = (p.0 - c.0, p.1 - c.1);
    match quarters % 4 {
        0 => p,
        1 => (c.0 - dy, c.1 + dx),
        2 => (c.0 - dx, c.1 - dy),
        _ => (c.0 + dy, c.1 - dx),
    }
}

/// The scene turned counterclockwise by `quarters` quarter turns about the point `c`.
pub fn rotate_scene(scene: &Scene, quarters: u8, c: (f64, f64)) -> Scene {
    let odd = quarters % 2 == 1;
    map_shapes(scene, |s| match s {
        Shape::Disc { x, y, r } => {
            let p = turn((*x, *y), c, quarters);
            Shape::Disc {
                x: p.0,
                y: p.1,
                r: *r,
            }
        }
        Shape::Rect { x, y, w, h } => {
            let p = turn((*x, *y), c, quarters);
            let (w, h) = if odd { (*h, *w) } else { (*w, *h) };
            Shape::Rect {
                x: p.0,
                y: p.1,
                w,
                h,
            }
        }
        Shape::Obround { x, y, w, h } => {
            let p = turn((*x, *y), c, quarters);
            let (w, h) = if odd { (*h, *w) } else { (*w, *h) };
            Shape::Obround {
                x: p.0,
                y: p.1,
                w,
                h,
            }
        }
        Shape::Stroke { x0, y0, x1, y1, w } => {
            let (a, b) = (turn((*x0, *y0), c, quarters), turn((*x1, *y1), c, quarters));
            Shape::Stroke {
                x0: a.0,
                y0: a.1,
                x1: b.0,
                y1: b.1,
                w: *w,
            }
        }
        Shape::Region { pts } => Shape::Region {
            pts: pts.iter().map(|p| turn(*p, c, quarters)).collect(),
        },
    })
}

pub fn rotate_holes(holes: &[Hole], quarters: u8, c: (f64, f64)) -> Vec<Hole> {
    holes
        .iter()
        .map(|h| {
            let p = turn((h.x_mm, h.y_mm), c, quarters);
            Hole {
                x_mm: p.0,
                y_mm: p.1,
                ..*h
            }
        })
        .collect()
}

fn shift(s: &Shape, dx: f64, dy: f64) -> Shape {
    match s {
        Shape::Disc { x, y, r } => Shape::Disc {
            x: x + dx,
            y: y + dy,
            r: *r,
        },
        Shape::Rect { x, y, w, h } => Shape::Rect {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
        },
        Shape::Obround { x, y, w, h } => Shape::Obround {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
        },
        Shape::Stroke { x0, y0, x1, y1, w } => Shape::Stroke {
            x0: x0 + dx,
            y0: y0 + dy,
            x1: x1 + dx,
            y1: y1 + dy,
            w: *w,
        },
        Shape::Region { pts } => Shape::Region {
            pts: pts.iter().map(|p| (p.0 + dx, p.1 + dy)).collect(),
        },
    }
}

/// Where the copies of a panel of `cols` x `rows` boards go: one every board's width (and height) plus the gap. The first copy stays where it is.
pub fn panel_offsets(
    b: &Bounds,
    cols: usize,
    rows: usize,
    gap_mm: f64,
) -> Result<Vec<(f64, f64)>, String> {
    if cols == 0 || rows == 0 || cols * rows > MAX_PANEL {
        return Err(format!(
            "a panel of {cols} x {rows}: it must have from 1 to {MAX_PANEL} boards"
        ));
    }
    if !(gap_mm.is_finite() && gap_mm >= 0.0) {
        return Err(format!(
            "the gap of the panel is {gap_mm}: it must be 0 or more"
        ));
    }
    if b.is_empty() {
        return Err("there is nothing to repeat: the layer is empty".into());
    }
    let (px, py) = (b.width() + gap_mm, b.height() + gap_mm);
    Ok((0..rows)
        .flat_map(|r| (0..cols).map(move |c| (c as f64 * px, r as f64 * py)))
        .collect())
}

pub fn panel_scene(scene: &Scene, offsets: &[(f64, f64)]) -> Scene {
    Scene {
        shapes: offsets
            .iter()
            .flat_map(|o| scene.shapes.iter().map(move |s| shift(s, o.0, o.1)))
            .collect(),
        notes: scene.notes.clone(),
    }
}

pub fn panel_holes(holes: &[Hole], offsets: &[(f64, f64)]) -> Vec<Hole> {
    offsets
        .iter()
        .flat_map(|o| {
            holes.iter().map(move |h| Hole {
                x_mm: h.x_mm + o.0,
                y_mm: h.y_mm + o.1,
                ..*h
            })
        })
        .collect()
}

/// Where the holes that hold the board go: outside the board (`Normal`), or in a fixed pattern about the middle of it (`Special`), for a clamp whose pins are a set distance apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolePattern {
    Normal,
    Special,
}

impl HolePattern {
    pub fn parse(v: &str) -> Result<HolePattern, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "normal" | "n" => Ok(HolePattern::Normal),
            "special" | "s" => Ok(HolePattern::Special),
            other => Err(format!(
                "alignment_pattern '{other}': it must be normal or special"
            )),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            HolePattern::Normal => "normal",
            HolePattern::Special => "special",
        }
    }
}

/// The holes that hold the board while it is turned over: two on either side of the board, as far from the line as each other (so the turned board finds them where they were),
/// and, when `count` is 3, one more on the line, which tells the board from the same board turned half a turn.
pub fn alignment_holes(
    b: &Bounds,
    diameter_mm: f64,
    margin_mm: f64,
    count: usize,
    axis: Axis,
    tool: u8,
) -> Result<Vec<Hole>, String> {
    alignment_pattern(
        b,
        diameter_mm,
        margin_mm,
        count,
        axis,
        tool,
        HolePattern::Normal,
        (40.0, 60.0),
    )
}

/// The holes of a pattern: 2, 3 or 6 of the normal kind, 2 or 6 of the special kind.
/// Normal: the pair is `margin_mm` outside the board, on the middle line; the third hole is on the line across; the six are the pair and the four corners, all `margin_mm` outside.
/// Special: the pair is `special.0` apart about the middle of the board; the six add four holes `special.1` apart across it, level with the two of the pair. Every pattern is the
/// same turned over, so the board finds the holes where they were.
#[allow(clippy::too_many_arguments)]
pub fn alignment_pattern(
    b: &Bounds,
    diameter_mm: f64,
    margin_mm: f64,
    count: usize,
    axis: Axis,
    tool: u8,
    pattern: HolePattern,
    special: (f64, f64),
) -> Result<Vec<Hole>, String> {
    if b.is_empty() {
        return Err("there is no board to put holes round".into());
    }
    if !(diameter_mm.is_finite() && diameter_mm > 0.0 && margin_mm.is_finite() && margin_mm > 0.0) {
        return Err(
            "the diameter and the margin of the alignment holes must be more than 0".into(),
        );
    }
    if !matches!(count, 2 | 3 | 6) {
        return Err(format!("{count} alignment holes: there must be 2, 3 or 6"));
    }
    let (cx, cy) = ((b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0);
    let at = |x: f64, y: f64| Hole {
        tool,
        x_mm: x,
        y_mm: y,
        diameter_mm,
    };
    if pattern == HolePattern::Special {
        if count == 3 {
            return Err("a special pattern has 2 or 6 holes".into());
        }
        let (a, c) = special;
        if !(a.is_finite() && a > 0.0 && c.is_finite() && c > 0.0) {
            return Err("the distances of the special pattern must be more than 0".into());
        }
        let pair = match axis {
            Axis::Vertical => [(cx - a / 2.0, cy), (cx + a / 2.0, cy)],
            Axis::Horizontal => [(cx, cy - a / 2.0), (cx, cy + a / 2.0)],
        };
        let mut holes: Vec<Hole> = pair.iter().map(|p| at(p.0, p.1)).collect();
        if count == 6 {
            for p in pair {
                match axis {
                    Axis::Vertical => {
                        holes.push(at(p.0, p.1 - c / 2.0));
                        holes.push(at(p.0, p.1 + c / 2.0));
                    }
                    Axis::Horizontal => {
                        holes.push(at(p.0 - c / 2.0, p.1));
                        holes.push(at(p.0 + c / 2.0, p.1));
                    }
                }
            }
        }
        return Ok(holes);
    }
    let mut holes = match axis {
        Axis::Vertical => vec![at(b.min_x - margin_mm, cy), at(b.max_x + margin_mm, cy)],
        Axis::Horizontal => vec![at(cx, b.min_y - margin_mm), at(cx, b.max_y + margin_mm)],
    };
    if count == 3 {
        holes.push(match axis {
            Axis::Vertical => at(cx, b.min_y - margin_mm),
            Axis::Horizontal => at(b.min_x - margin_mm, cy),
        });
    }
    if count == 6 {
        for x in [b.min_x - margin_mm, b.max_x + margin_mm] {
            for y in [b.min_y - margin_mm, b.max_y + margin_mm] {
                holes.push(at(x, y));
            }
        }
    }
    Ok(holes)
}

/// How a board is laid out for milling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Quarter turns counterclockwise, 0 to 3.
    pub quarters: u8,
    pub mirror: Option<Axis>,
    pub cols: usize,
    pub rows: usize,
    pub gap_mm: f64,
}

impl Default for Layout {
    fn default() -> Self {
        Layout {
            quarters: 0,
            mirror: None,
            cols: 1,
            rows: 1,
            gap_mm: 2.0,
        }
    }
}

/// The copper, the holes and the closed paths of the outline of a board, which are moved together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Board {
    pub scene: Scene,
    pub holes: Vec<Hole>,
    pub outline: Vec<Vec<(f64, f64)>>,
}

impl Board {
    /// The box the layout is worked out from: the outline when there is one, else the copper, else the holes.
    pub fn reference(&self) -> Bounds {
        let mut b = Bounds::empty();
        for p in self.outline.iter().flatten() {
            b.include(p.0, p.1);
        }
        if b.is_empty() {
            b = self.scene.bounds();
        }
        if b.is_empty() {
            for h in &self.holes {
                b.include(h.x_mm, h.y_mm);
            }
        }
        b
    }
}

/// Lays the board out: turned, turned over and repeated on a panel. Returns the board and the box of the whole panel.
pub fn lay_out(b: &Board, l: &Layout) -> Result<(Board, Bounds), String> {
    if l.quarters > 3 {
        return Err(format!(
            "{} quarter turns: it must be from 0 to 3",
            l.quarters
        ));
    }
    let r0 = b.reference();
    if r0.is_empty() {
        return Err("there is nothing to lay out".into());
    }
    let mut cur = b.clone();
    if l.quarters > 0 {
        let c = ((r0.min_x + r0.max_x) / 2.0, (r0.min_y + r0.max_y) / 2.0);
        cur = Board {
            scene: rotate_scene(&cur.scene, l.quarters, c),
            holes: rotate_holes(&cur.holes, l.quarters, c),
            outline: cur
                .outline
                .iter()
                .map(|p| p.iter().map(|q| turn(*q, c, l.quarters)).collect())
                .collect(),
        };
    }
    let r1 = cur.reference();
    if let Some(axis) = l.mirror {
        let at = axis.middle(&r1);
        cur = Board {
            scene: mirror_scene(&cur.scene, axis, at),
            holes: mirror_holes(&cur.holes, axis, at),
            outline: cur
                .outline
                .iter()
                .map(|p| p.iter().map(|q| axis.flip(at, *q)).collect())
                .collect(),
        };
    }
    let r2 = cur.reference();
    let offsets = panel_offsets(&r2, l.cols, l.rows, l.gap_mm)?;
    let board = Board {
        scene: panel_scene(&cur.scene, &offsets),
        holes: panel_holes(&cur.holes, &offsets),
        outline: offsets
            .iter()
            .flat_map(|o| {
                cur.outline
                    .iter()
                    .map(move |p| p.iter().map(|q| (q.0 + o.0, q.1 + o.1)).collect())
            })
            .collect(),
    };
    let last = offsets[offsets.len() - 1];
    let whole = Bounds {
        min_x: r2.min_x,
        min_y: r2.min_y,
        max_x: r2.max_x + last.0,
        max_y: r2.max_y + last.1,
    };
    Ok((board, whole))
}
