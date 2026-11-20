//! The aperture macros (`AM`) evaluated: the primitives of a macro for the parameters of one aperture.
//!
//! [`evaluate`] reads the lines of a [`MacroDefinition`] and gives the primitives as plain data, in the units
//! of the file and with every variable and expression already calculated. It reads all the primitives of the
//! standard: circle (1), vector line (20 and its old form 2), centre line (21), lower-left line (22), outline
//! (4), regular polygon (5), moire (6) and thermal (7), the comments (0), the variables (`$3=$1x2`) and the
//! arithmetic (`+`, `-`, `x`, `X`, `*`, `/`, parentheses and signs).
//!
//! It never panics and it is bounded: a nesting of more than 64 levels, an expression of more than 1024
//! characters, more than 4096 lines or primitives, an outline of more than 4096 corners and a variable
//! above `$1024` are not read, and the reason is listed in [`Evaluated::skipped`].
//!
//! The shapes are not calculated here (no polygons, no union): [`Primitive::bounds`] gives the box and the
//! drawing belongs to the geometry crate.

use crate::MacroDefinition;
use std::collections::BTreeMap;

/// The most primitives one macro may produce.
pub const MAX_PRIMITIVES: usize = 4096;
/// The most corners of an outline or a regular polygon.
pub const MAX_VERTICES: usize = 4096;
/// The deepest nesting of signs and parentheses in an expression.
pub const MAX_DEPTH: usize = 64;
/// The longest expression (in characters) that is read.
pub const MAX_EXPRESSION: usize = 1024;
/// The most lines of a macro that are read.
pub const MAX_LINES: usize = 4096;
/// The highest variable number that is kept.
pub const MAX_VARIABLE: u32 = 1024;

/// One primitive of a macro with its numbers calculated.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// Circle (1).
    Circle {
        /// True to add copper, false to clear.
        dark: bool,
        /// Diameter.
        diameter: f64,
        /// Centre.
        center: (f64, f64),
        /// Rotation of the centre about the origin of the macro, in degrees.
        rotation: f64,
    },
    /// Vector line (20, or 2 in the old files): a line of a width between two points with square ends.
    VectorLine {
        /// True to add copper, false to clear.
        dark: bool,
        /// Width.
        width: f64,
        /// Start.
        start: (f64, f64),
        /// End.
        end: (f64, f64),
        /// Rotation about the origin of the macro, in degrees.
        rotation: f64,
    },
    /// Centre line (21): a rectangle given by its centre.
    CenterLine {
        /// True to add copper, false to clear.
        dark: bool,
        /// Size along X before the rotation.
        width: f64,
        /// Size along Y before the rotation.
        height: f64,
        /// Centre.
        center: (f64, f64),
        /// Rotation about the origin of the macro, in degrees.
        rotation: f64,
    },
    /// Lower-left line (22): a rectangle given by its lower-left corner.
    LowerLeftLine {
        /// True to add copper, false to clear.
        dark: bool,
        /// Size along X before the rotation.
        width: f64,
        /// Size along Y before the rotation.
        height: f64,
        /// Lower-left corner.
        lower_left: (f64, f64),
        /// Rotation about the origin of the macro, in degrees.
        rotation: f64,
    },
    /// Outline (4): a closed polygon (the last point repeats the first).
    Outline {
        /// True to add copper, false to clear.
        dark: bool,
        /// The corners, with the first one not repeated at the end.
        points: Vec<(f64, f64)>,
        /// Rotation about the origin of the macro, in degrees.
        rotation: f64,
    },
    /// Regular polygon (5).
    Polygon {
        /// True to add copper, false to clear.
        dark: bool,
        /// Number of corners.
        vertices: u32,
        /// Centre.
        center: (f64, f64),
        /// Diameter of the circle through the corners.
        diameter: f64,
        /// Rotation about the origin of the macro, in degrees.
        rotation: f64,
    },
    /// Moire (6): concentric rings with a cross. Always adds copper.
    Moire {
        /// Centre.
        center: (f64, f64),
        /// Outer diameter of the first ring.
        outer_diameter: f64,
        /// Thickness of each ring.
        ring_thickness: f64,
        /// Gap between rings.
        gap: f64,
        /// The most rings.
        max_rings: u32,
        /// Thickness of the lines of the cross.
        crosshair_thickness: f64,
        /// Length of the lines of the cross.
        crosshair_length: f64,
        /// Rotation about the origin of the macro, in degrees.
        rotation: f64,
    },
    /// Thermal (7): a ring cut by a cross of gaps. Always adds copper.
    Thermal {
        /// Centre.
        center: (f64, f64),
        /// Outer diameter.
        outer_diameter: f64,
        /// Inner diameter.
        inner_diameter: f64,
        /// Width of the gaps of the cross.
        gap: f64,
        /// Rotation about the origin of the macro, in degrees.
        rotation: f64,
    },
}

/// What a macro gave for a set of parameters and what could not be read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Evaluated {
    /// The primitives, in the order of the macro.
    pub primitives: Vec<Primitive>,
    /// For each line left out, its position in the body of the macro (from 0) and the reason.
    pub skipped: Vec<(usize, &'static str)>,
}

impl Evaluated {
    /// True when every line of the macro was read.
    pub fn is_complete(&self) -> bool {
        self.skipped.is_empty()
    }
}

struct Expr<'a> {
    chars: Vec<char>,
    pos: usize,
    vars: &'a BTreeMap<u32, f64>,
    depth: usize,
}

impl Expr<'_> {
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
        self.depth += 1;
        let v = if self.depth > MAX_DEPTH {
            None
        } else {
            self.factor_inside()
        };
        self.depth -= 1;
        v
    }
    fn digits(&mut self, allow_point: bool) -> String {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || (allow_point && c == '.'))
        {
            self.pos += 1;
        }
        self.chars[start..self.pos].iter().collect()
    }
    fn factor_inside(&mut self) -> Option<f64> {
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
                let n: u32 = self.digits(false).parse().ok()?;
                // a variable never given is zero
                Some(self.vars.get(&n).copied().unwrap_or(0.0))
            }
            c if c.is_ascii_digit() || c == '.' => self.digits(true).parse().ok(),
            _ => None,
        }
    }
}

/// The value of an arithmetic expression of a macro with its variables, or `None` when it does not parse,
/// divides by zero, is too deep or too long, or is not a finite number.
pub fn eval_expression(text: &str, variables: &BTreeMap<u32, f64>) -> Option<f64> {
    let chars: Vec<char> = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .take(MAX_EXPRESSION + 1)
        .collect();
    if chars.len() > MAX_EXPRESSION {
        return None;
    }
    let mut p = Expr {
        chars,
        pos: 0,
        vars: variables,
        depth: 0,
    };
    let v = p.expr()?;
    (p.pos == p.chars.len() && v.is_finite()).then_some(v)
}

/// The primitives of `def` for the parameters `params` (`$1`, `$2`, ... of the aperture definition).
pub fn evaluate(def: &MacroDefinition, params: &[f64]) -> Evaluated {
    let mut out = Evaluated::default();
    let mut vars: BTreeMap<u32, f64> = BTreeMap::new();
    for (i, v) in params.iter().enumerate().take(MAX_VARIABLE as usize) {
        vars.insert(i as u32 + 1, *v);
    }
    for (index, line) in def.body.iter().enumerate() {
        if index >= MAX_LINES {
            out.skipped.push((index, "a macro with too many lines"));
            break;
        }
        let text: String = line.chars().filter(|c| !c.is_whitespace()).collect();
        if text.is_empty() {
            continue;
        }
        // a comment: the primitive 0 and any text
        if text.starts_with('0') && !text.starts_with("0,") && !text.starts_with("0.") {
            continue;
        }
        if let Some(rest) = text.strip_prefix('$') {
            let assigned = rest.split_once('=').and_then(|(n, value)| {
                let n: u32 = n.parse().ok()?;
                ((1..=MAX_VARIABLE).contains(&n)).then_some(())?;
                Some((n, eval_expression(value, &vars)?))
            });
            match assigned {
                Some((n, v)) => {
                    vars.insert(n, v);
                }
                None => out
                    .skipped
                    .push((index, "an assignment that cannot be read")),
            }
            continue;
        }
        if out.primitives.len() >= MAX_PRIMITIVES {
            out.skipped
                .push((index, "a macro with too many primitives"));
            break;
        }
        match primitive(&text, &vars) {
            Ok(p) => out.primitives.push(p),
            Err(reason) => out.skipped.push((index, reason)),
        }
    }
    out
}

fn primitive(text: &str, vars: &BTreeMap<u32, f64>) -> Result<Primitive, &'static str> {
    let mut fields = text.split(',');
    let code: u32 = fields
        .next()
        .and_then(|c| c.parse().ok())
        .ok_or("a primitive that cannot be read")?;
    let values: Option<Vec<f64>> = fields.map(|f| eval_expression(f, vars)).collect();
    let v = values.ok_or("a primitive with an expression that cannot be read")?;
    let get = |i: usize| v.get(i).copied().unwrap_or(0.0);
    let arity = |min: usize, max: usize| -> Result<(), &'static str> {
        if v.len() < min {
            Err("a primitive with too few values")
        } else if v.len() > max {
            Err("a primitive with too many values")
        } else {
            Ok(())
        }
    };
    let exposure = || -> Result<bool, &'static str> {
        match get(0) {
            1.0 => Ok(true),
            0.0 => Ok(false),
            _ => Err("an exposure that is not 0 or 1"),
        }
    };
    match code {
        1 => {
            arity(4, 5)?;
            Ok(Primitive::Circle {
                dark: exposure()?,
                diameter: get(1),
                center: (get(2), get(3)),
                rotation: get(4),
            })
        }
        2 | 20 => {
            arity(7, 7)?;
            Ok(Primitive::VectorLine {
                dark: exposure()?,
                width: get(1),
                start: (get(2), get(3)),
                end: (get(4), get(5)),
                rotation: get(6),
            })
        }
        21 => {
            arity(5, 6)?;
            Ok(Primitive::CenterLine {
                dark: exposure()?,
                width: get(1),
                height: get(2),
                center: (get(3), get(4)),
                rotation: get(5),
            })
        }
        22 => {
            arity(5, 6)?;
            Ok(Primitive::LowerLeftLine {
                dark: exposure()?,
                width: get(1),
                height: get(2),
                lower_left: (get(3), get(4)),
                rotation: get(5),
            })
        }
        4 => {
            arity(6, 5 + 2 * (MAX_VERTICES + 1))?;
            let subsequent = get(1);
            if subsequent.fract() != 0.0 || !(2.0..=MAX_VERTICES as f64).contains(&subsequent) {
                return Err("an outline with a number of points that cannot be read");
            }
            let n = subsequent as usize;
            // exposure, the count, the first point, `n` more points and the rotation
            if v.len() != 2 + 2 * (n + 1) + 1 {
                return Err("an outline whose number of values does not fit its points");
            }
            let mut points: Vec<(f64, f64)> =
                (0..=n).map(|i| (get(2 + 2 * i), get(3 + 2 * i))).collect();
            if points.first() == points.last() {
                points.pop();
            }
            Ok(Primitive::Outline {
                dark: exposure()?,
                points,
                rotation: get(2 + 2 * (n + 1)),
            })
        }
        5 => {
            arity(5, 6)?;
            let n = get(1);
            if n.fract() != 0.0 || !(3.0..=MAX_VERTICES as f64).contains(&n) {
                return Err("a polygon whose corners are not 3 or more");
            }
            Ok(Primitive::Polygon {
                dark: exposure()?,
                vertices: n as u32,
                center: (get(2), get(3)),
                diameter: get(4),
                rotation: get(5),
            })
        }
        6 => {
            arity(9, 9)?;
            let rings = get(5);
            if rings.fract() != 0.0 || !(0.0..=MAX_VERTICES as f64).contains(&rings) {
                return Err("a moire with a number of rings that cannot be read");
            }
            Ok(Primitive::Moire {
                center: (get(0), get(1)),
                outer_diameter: get(2),
                ring_thickness: get(3),
                gap: get(4),
                max_rings: rings as u32,
                crosshair_thickness: get(6),
                crosshair_length: get(7),
                rotation: get(8),
            })
        }
        7 => {
            arity(6, 6)?;
            Ok(Primitive::Thermal {
                center: (get(0), get(1)),
                outer_diameter: get(2),
                inner_diameter: get(3),
                gap: get(4),
                rotation: get(5),
            })
        }
        _ => Err("a primitive that is not part of the standard"),
    }
}

fn rotate(p: (f64, f64), degrees: f64) -> (f64, f64) {
    if degrees == 0.0 {
        return p;
    }
    let (s, c) = degrees.to_radians().sin_cos();
    (p.0 * c - p.1 * s, p.0 * s + p.1 * c)
}

fn extend(b: &mut Option<[f64; 4]>, p: (f64, f64)) {
    match b {
        Some(r) => {
            r[0] = r[0].min(p.0);
            r[1] = r[1].min(p.1);
            r[2] = r[2].max(p.0);
            r[3] = r[3].max(p.1);
        }
        None => *b = Some([p.0, p.1, p.0, p.1]),
    }
}

impl Primitive {
    /// True when the primitive adds copper and false when it clears (moire and thermal always add).
    pub fn is_dark(&self) -> bool {
        match self {
            Primitive::Circle { dark, .. }
            | Primitive::VectorLine { dark, .. }
            | Primitive::CenterLine { dark, .. }
            | Primitive::LowerLeftLine { dark, .. }
            | Primitive::Outline { dark, .. }
            | Primitive::Polygon { dark, .. } => *dark,
            Primitive::Moire { .. } | Primitive::Thermal { .. } => true,
        }
    }

    /// The corners of the primitive when it is a polygon with straight sides (lines, rectangles, outlines and
    /// regular polygons), with the rotation applied, in the units of the macro. A circle, a moire and a thermal
    /// have no corners and give `None`; use [`Primitive::bounds`] for them.
    pub fn corners(&self) -> Option<Vec<(f64, f64)>> {
        match self {
            Primitive::VectorLine {
                width,
                start,
                end,
                rotation,
                ..
            } => {
                let (dx, dy) = (end.0 - start.0, end.1 - start.1);
                let len = dx.hypot(dy);
                let (nx, ny) = if len > 0.0 {
                    (-dy / len * width / 2.0, dx / len * width / 2.0)
                } else {
                    (0.0, 0.0)
                };
                Some(
                    [
                        (start.0 + nx, start.1 + ny),
                        (end.0 + nx, end.1 + ny),
                        (end.0 - nx, end.1 - ny),
                        (start.0 - nx, start.1 - ny),
                    ]
                    .iter()
                    .map(|p| rotate(*p, *rotation))
                    .collect(),
                )
            }
            Primitive::CenterLine {
                width,
                height,
                center,
                rotation,
                ..
            } => Some(rectangle(center.0, center.1, *width, *height, *rotation)),
            Primitive::LowerLeftLine {
                width,
                height,
                lower_left,
                rotation,
                ..
            } => Some(rectangle(
                lower_left.0 + width / 2.0,
                lower_left.1 + height / 2.0,
                *width,
                *height,
                *rotation,
            )),
            Primitive::Outline {
                points, rotation, ..
            } => Some(points.iter().map(|p| rotate(*p, *rotation)).collect()),
            Primitive::Polygon {
                vertices,
                center,
                diameter,
                rotation,
                ..
            } => Some(
                (0..*vertices)
                    .map(|i| {
                        let a = (360.0 * f64::from(i) / f64::from(*vertices)).to_radians();
                        rotate(
                            (
                                center.0 + diameter / 2.0 * a.cos(),
                                center.1 + diameter / 2.0 * a.sin(),
                            ),
                            *rotation,
                        )
                    })
                    .collect(),
            ),
            _ => None,
        }
    }

    /// The box `[min x, min y, max x, max y]` that holds the primitive, in the units of the macro, or `None`
    /// for a primitive of no size. A circle, a moire and a thermal are held by the box of their outer circle
    /// (the centre rotated about the origin).
    pub fn bounds(&self) -> Option<[f64; 4]> {
        let mut b = None;
        if let Some(corners) = self.corners() {
            for p in corners {
                extend(&mut b, p);
            }
            return b;
        }
        let (center, radius, rotation) = match self {
            Primitive::Circle {
                diameter,
                center,
                rotation,
                ..
            } => (*center, diameter.abs() / 2.0, *rotation),
            Primitive::Moire {
                center,
                outer_diameter,
                crosshair_length,
                rotation,
                ..
            } => (
                *center,
                outer_diameter.abs().max(crosshair_length.abs()) / 2.0,
                *rotation,
            ),
            Primitive::Thermal {
                center,
                outer_diameter,
                rotation,
                ..
            } => (*center, outer_diameter.abs() / 2.0, *rotation),
            _ => return None,
        };
        let c = rotate(center, rotation);
        extend(&mut b, (c.0 - radius, c.1 - radius));
        extend(&mut b, (c.0 + radius, c.1 + radius));
        b
    }
}

fn rectangle(cx: f64, cy: f64, w: f64, h: f64, rotation: f64) -> Vec<(f64, f64)> {
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .iter()
        .map(|(sx, sy)| rotate((cx + sx * w / 2.0, cy + sy * h / 2.0), rotation))
        .collect()
}

/// The box `[min x, min y, max x, max y]` of all the primitives of an evaluated macro, or `None` when it has
/// no primitive of size.
pub fn bounds(evaluated: &Evaluated) -> Option<[f64; 4]> {
    let mut b: Option<[f64; 4]> = None;
    for r in evaluated.primitives.iter().filter_map(Primitive::bounds) {
        extend(&mut b, (r[0], r[1]));
        extend(&mut b, (r[2], r[3]));
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(lines: &[&str]) -> MacroDefinition {
        MacroDefinition {
            name: "M".into(),
            body: lines.iter().map(|l| (*l).to_owned()).collect(),
        }
    }

    #[test]
    fn variables_and_arithmetic_are_calculated() {
        let e = evaluate(&def(&["$3=$1x2+1", "1,1,$3,($2/2),-$1"]), &[0.5, 4.0]);
        assert!(e.is_complete(), "{:?}", e.skipped);
        assert_eq!(
            e.primitives,
            vec![Primitive::Circle {
                dark: true,
                diameter: 2.0,
                center: (2.0, -0.5),
                rotation: 0.0
            }]
        );
    }

    #[test]
    fn every_primitive_of_the_standard_is_read() {
        let e = evaluate(
            &def(&[
                "0 a comment",
                "1,1,1.5,0,0",
                "20,1,0.2,0,0,1,1,45",
                "2,1,0.2,0,0,1,1,0",
                "21,0,1,0.5,0,0,0",
                "22,1,1,0.5,-0.5,-0.25,0",
                "4,1,3,0,0,1,0,1,1,0,0,0",
                "5,1,6,0,0,2,0",
                "6,0,0,3,0.2,0.1,4,0.05,3.5,0",
                "7,0,0,2,1,0.2,0",
            ]),
            &[],
        );
        assert!(e.is_complete(), "{:?}", e.skipped);
        assert_eq!(e.primitives.len(), 9);
        assert!(matches!(
            e.primitives[3],
            Primitive::CenterLine { dark: false, .. }
        ));
        match &e.primitives[5] {
            Primitive::Outline { points, .. } => {
                assert_eq!(points.len(), 3, "the closing point is dropped")
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            e.primitives[7],
            Primitive::Moire { max_rings: 4, .. }
        ));
        assert!(matches!(e.primitives[8], Primitive::Thermal { .. }));
        assert!(e.primitives[7].is_dark() && e.primitives[8].is_dark());
    }

    #[test]
    fn what_cannot_be_read_is_listed_with_its_line_and_the_rest_is_kept() {
        let e = evaluate(
            &def(&[
                "1,1,1,0,0",
                "1,1,1/0,0,0",
                "99,1,1",
                "1,2,1,0,0",
                "4,1,3,0,0,1,0,1,1,0",
                "5,1,2,0,0,2,0",
                "$0=3",
                "1,1,1,0",
                "1,1,1,0,0",
            ]),
            &[],
        );
        assert_eq!(e.primitives.len(), 2);
        let lines: Vec<usize> = e.skipped.iter().map(|s| s.0).collect();
        assert_eq!(lines, vec![1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn the_limits_of_a_hostile_macro_hold() {
        let deep = format!("1,1,{}1{},0,0", "(".repeat(5000), ")".repeat(5000));
        assert!(!evaluate(&def(&[&deep]), &[]).is_complete());
        let signs = format!("1,1,{}1,0,0", "-".repeat(100_000));
        assert!(!evaluate(&def(&[&signs]), &[]).is_complete());
        let many: Vec<String> = (0..10_000).map(|_| "1,1,1,0,0".to_owned()).collect();
        let lines: Vec<&str> = many.iter().map(String::as_str).collect();
        let e = evaluate(&def(&lines), &[]);
        assert_eq!(e.primitives.len(), MAX_PRIMITIVES);
        assert!(!e.is_complete());
        let big_outline = format!("4,1,{},0,0,{}0", 100_000, "1,1,".repeat(100_000));
        assert!(!evaluate(&def(&[&big_outline]), &[]).is_complete());
        assert_eq!(eval_expression("2x(3+1)/4", &BTreeMap::new()), Some(2.0));
        assert_eq!(eval_expression("1e5", &BTreeMap::new()), None);
        assert_eq!(eval_expression("", &BTreeMap::new()), None);
    }

    #[test]
    fn the_bounds_follow_the_rotation_about_the_origin_of_the_macro() {
        let e = evaluate(&def(&["21,1,2,1,0,0,90"]), &[]);
        let b = bounds(&e).unwrap();
        assert!((b[0] + 0.5).abs() < 1e-12 && (b[2] - 0.5).abs() < 1e-12);
        assert!((b[1] + 1.0).abs() < 1e-12 && (b[3] - 1.0).abs() < 1e-12);
        let c = evaluate(&def(&["1,1,2,1,0,90"]), &[]);
        let b = bounds(&c).unwrap();
        assert!((b[1] - 0.0).abs() < 1e-12 && (b[3] - 2.0).abs() < 1e-12);
        assert_eq!(bounds(&Evaluated::default()), None);
    }

    #[test]
    fn a_vector_line_has_square_ends_and_a_regular_polygon_starts_at_angle_zero() {
        let e = evaluate(&def(&["20,1,2,0,0,4,0,0", "5,1,4,0,0,2,0"]), &[]);
        let line = e.primitives[0].corners().unwrap();
        assert_eq!(line.len(), 4);
        let b = e.primitives[0].bounds().unwrap();
        assert_eq!(b, [0.0, -1.0, 4.0, 1.0]);
        let sq = e.primitives[1].corners().unwrap();
        assert!((sq[0].0 - 1.0).abs() < 1e-12 && sq[0].1.abs() < 1e-12);
        assert!(Primitive::Circle {
            dark: true,
            diameter: 1.0,
            center: (0.0, 0.0),
            rotation: 0.0
        }
        .corners()
        .is_none());
    }
}
