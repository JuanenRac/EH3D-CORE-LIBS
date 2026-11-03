//! The shapes of an aperture macro (`AM`): the primitives that make the pads of most boards,
//! drawn at the point of a flash.
//!
//! Drawn: circle (1), vector line (20), center line (21), lower-left line (22), outline (4) and
//! regular polygon (5), with variables (`$1`, `$3=$1x2`), arithmetic and rotation. A primitive
//! that clears (exposure 0), a moire (6), a thermal (7) and anything unknown are left out and
//! reported, so the caller can say the pad is incomplete.

use crate::scene::Shape;
use eh3d_parser_gerber::MacroDefinition;

/// The most primitives one flash of a macro may produce.
const MAX_PRIMITIVES: usize = 4096;
/// The most corners of an outline or a regular polygon.
const MAX_VERTICES: usize = 4096;

/// What a macro drew at a point and what it could not draw.
#[derive(Debug, Default)]
pub(crate) struct MacroShapes {
    pub shapes: Vec<Shape>,
    /// The reason of each primitive that was left out.
    pub skipped: Vec<&'static str>,
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    vars: &'a [f64],
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn expr(&mut self) -> Option<f64> {
        let mut v = self.term()?;
        while let Some(c) = self.peek() {
            match c {
                '+' => {
                    self.pos += 1;
                    v += self.term()?;
                }
                '-' => {
                    self.pos += 1;
                    v -= self.term()?;
                }
                _ => break,
            }
        }
        Some(v)
    }
    fn term(&mut self) -> Option<f64> {
        let mut v = self.factor()?;
        while let Some(c) = self.peek() {
            match c {
                'x' | 'X' | '*' => {
                    self.pos += 1;
                    v *= self.factor()?;
                }
                '/' => {
                    self.pos += 1;
                    let d = self.factor()?;
                    if d == 0.0 {
                        return None;
                    }
                    v /= d;
                }
                _ => break,
            }
        }
        Some(v)
    }
    fn factor(&mut self) -> Option<f64> {
        match self.peek()? {
            '-' => {
                self.pos += 1;
                Some(-self.factor()?)
            }
            '+' => {
                self.pos += 1;
                self.factor()
            }
            '(' => {
                self.pos += 1;
                let v = self.expr()?;
                if self.peek()? != ')' {
                    return None;
                }
                self.pos += 1;
                Some(v)
            }
            '$' => {
                self.pos += 1;
                let start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    self.pos += 1;
                }
                let n: usize = self.chars[start..self.pos]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()?;
                // a variable never given is zero
                Some(self.vars.get(n.checked_sub(1)?).copied().unwrap_or(0.0))
            }
            c if c.is_ascii_digit() || c == '.' => {
                let start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                    self.pos += 1;
                }
                self.chars[start..self.pos]
                    .iter()
                    .collect::<String>()
                    .parse()
                    .ok()
            }
            _ => None,
        }
    }
}

/// The value of an arithmetic expression with variables, or `None` when it does not parse.
pub(crate) fn eval(text: &str, vars: &[f64]) -> Option<f64> {
    let mut p = Parser {
        chars: text.chars().filter(|c| !c.is_whitespace()).collect(),
        pos: 0,
        vars,
    };
    let v = p.expr()?;
    (p.pos == p.chars.len() && v.is_finite()).then_some(v)
}

fn rotate(p: (f64, f64), degrees: f64) -> (f64, f64) {
    if degrees == 0.0 {
        return p;
    }
    let (s, c) = degrees.to_radians().sin_cos();
    (p.0 * c - p.1 * s, p.0 * s + p.1 * c)
}

fn quad(cx: f64, cy: f64, w: f64, h: f64, rot: f64) -> Vec<(f64, f64)> {
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .iter()
        .map(|(sx, sy)| rotate((cx + sx * w / 2.0, cy + sy * h / 2.0), rot))
        .collect()
}

/// The shapes of one flash of `def` with the parameters `params`, in the units of the file,
/// moved to `at` and scaled by `k` (to millimetres).
pub(crate) fn draw(def: &MacroDefinition, params: &[f64], at: (f64, f64), k: f64) -> MacroShapes {
    let mut out = MacroShapes::default();
    let mut vars: Vec<f64> = params.to_vec();
    let place = |pts: Vec<(f64, f64)>, out: &mut MacroShapes| {
        out.shapes.push(Shape::Region {
            pts: pts
                .into_iter()
                .map(|(x, y)| (at.0 + x * k, at.1 + y * k))
                .collect(),
        });
    };
    for line in &def.body {
        let text: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if text.is_empty() {
            continue;
        }
        // a comment: a primitive 0 and any text
        if text == "0" || (text.starts_with('0') && !text.starts_with("0,") && text.len() > 1) {
            continue;
        }
        if let Some(rest) = text.strip_prefix('$') {
            let Some((n, value)) = rest.split_once('=') else {
                out.skipped.push("an assignment that cannot be read");
                continue;
            };
            let (Ok(n), Some(v)) = (n.parse::<usize>(), eval(value, &vars)) else {
                out.skipped.push("an assignment that cannot be read");
                continue;
            };
            if n == 0 || n > 1024 {
                out.skipped.push("an assignment that cannot be read");
                continue;
            }
            if vars.len() < n {
                vars.resize(n, 0.0);
            }
            vars[n - 1] = v;
            continue;
        }
        if out.shapes.len() >= MAX_PRIMITIVES {
            out.skipped.push("a macro with too many primitives");
            break;
        }
        let fields: Vec<&str> = text.split(',').collect();
        let values: Option<Vec<f64>> = fields.iter().skip(1).map(|f| eval(f, &vars)).collect();
        let (Some(code), Some(v)) = (fields.first().and_then(|c| c.parse::<u32>().ok()), values)
        else {
            out.skipped.push("a primitive that cannot be read");
            continue;
        };
        // every drawn primitive starts with its exposure: 1 draws, 0 clears
        if matches!(code, 1 | 4 | 5 | 20 | 21 | 22) && v.first().copied() != Some(1.0) {
            out.skipped.push("a primitive that clears");
            continue;
        }
        let get = |i: usize| v.get(i).copied().unwrap_or(0.0);
        match code {
            1 if v.len() >= 4 => {
                let (c, d) = (rotate((get(2), get(3)), get(4)), get(1));
                out.shapes.push(Shape::Disc {
                    x: at.0 + c.0 * k,
                    y: at.1 + c.1 * k,
                    r: d * k / 2.0,
                });
            }
            20 if v.len() >= 7 => {
                let (a, b) = ((get(2), get(3)), (get(4), get(5)));
                let w = get(1);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.0 && w > 0.0 {
                    let (nx, ny) = (-dy / len * w / 2.0, dx / len * w / 2.0);
                    let pts = [
                        (a.0 + nx, a.1 + ny),
                        (b.0 + nx, b.1 + ny),
                        (b.0 - nx, b.1 - ny),
                        (a.0 - nx, a.1 - ny),
                    ]
                    .iter()
                    .map(|p| rotate(*p, get(6)))
                    .collect();
                    place(pts, &mut out);
                }
            }
            21 if v.len() >= 6 => place(quad(get(3), get(4), get(1), get(2), get(5)), &mut out),
            22 if v.len() >= 6 => place(
                quad(
                    get(3) + get(1) / 2.0,
                    get(4) + get(2) / 2.0,
                    get(1),
                    get(2),
                    get(5),
                ),
                &mut out,
            ),
            4 if v.len() >= 4 => {
                let n = get(1);
                if !(3.0..=MAX_VERTICES as f64).contains(&n) || v.len() < 4 + 2 * n as usize {
                    out.skipped.push("an outline that cannot be read");
                    continue;
                }
                let rot = get(4 + 2 * n as usize);
                // the last point repeats the first
                let pts = (0..n as usize)
                    .map(|i| rotate((get(2 + 2 * i), get(3 + 2 * i)), rot))
                    .collect();
                place(pts, &mut out);
            }
            5 if v.len() >= 6 => {
                let n = get(1);
                if !(3.0..=MAX_VERTICES as f64).contains(&n) {
                    out.skipped.push("a polygon that cannot be read");
                    continue;
                }
                let (c, d, rot) = ((get(2), get(3)), get(4), get(5));
                let pts = (0..n as usize)
                    .map(|i| {
                        let a = (rot + 360.0 * i as f64 / n).to_radians();
                        (c.0 + d / 2.0 * a.cos(), c.1 + d / 2.0 * a.sin())
                    })
                    .collect();
                place(pts, &mut out);
            }
            1 | 4 | 5 | 20 | 21 | 22 => out.skipped.push("a primitive with too few values"),
            _ => out.skipped.push("a primitive that is not drawn"),
        }
    }
    out
}
