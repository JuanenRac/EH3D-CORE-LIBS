//! The drawing of a Gerber file with its blocks, repeats and transforms put in place.
//!
//! [`parse`](crate::parse) gives the commands of a file as they are written: a flash of a block aperture is one
//! command, a step and repeat is a pair of commands, `LM`/`LR`/`LS` are commands that change what comes after, and
//! the parameters of the whole image (`OF`, `SF`, `MI`, `IR`, `AS`) are values. [`resolve`] walks the commands and
//! returns a flat list of [`Item`]s whose coordinates are final: every copy of a repeat, every flash of a block
//! with the content of the block placed, the arcs with their centre, the regions with their contours. A program that
//! draws or machines a layer needs only this list.
//!
//! What it does **not** do: it does not draw (the apertures stay numbers: [`Shape::Flash`] says which aperture and
//! where), it does not compose the polarity (each item says whether it is dark or clear and the group it belongs to)
//! and it does not flatten arcs (see [`crate::arc`]).
//!
//! # Groups
//!
//! A block aperture is composed on its own and then added to the layer with the polarity it was flashed with. The
//! items of one flash of a block share a [`Group`]: draw the items of a group in order (dark adds, clear removes
//! inside the group), and then add the result to the layer as the group's polarity says. A group may be inside
//! another group (`parent`). An item with `group == None` belongs to the layer.
//!
//! # Limits
//!
//! The list stops at [`ResolveLimits::max_items`] with an `image-too-large` error, a block aperture nested deeper
//! than [`ResolveLimits::max_depth`] or that draws itself is left out with a `block-aperture-cycle` error, and
//! nothing here panics.

use crate::arc::arc_geometry;
use crate::{
    codes, macros, Aperture, Command, Diagnostic, GerberFile, Mirror, Operation, Polarity,
    Severity, Units,
};

/// A transformation of the plane: `x' = a*x + c*y + e` and `y' = b*x + d*y + f`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine {
    /// Coefficient of x in x'.
    pub a: f64,
    /// Coefficient of x in y'.
    pub b: f64,
    /// Coefficient of y in x'.
    pub c: f64,
    /// Coefficient of y in y'.
    pub d: f64,
    /// Shift in x.
    pub e: f64,
    /// Shift in y.
    pub f: f64,
}

impl Affine {
    /// The transformation that moves nothing.
    pub const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// A shift.
    pub fn translate(x: f64, y: f64) -> Affine {
        Affine {
            e: x,
            f: y,
            ..Affine::IDENTITY
        }
    }

    /// A scale about the origin.
    pub fn scale(sx: f64, sy: f64) -> Affine {
        Affine {
            a: sx,
            d: sy,
            ..Affine::IDENTITY
        }
    }

    /// A rotation about the origin, counterclockwise and in degrees. The right angles are exact.
    pub fn rotate_degrees(degrees: f64) -> Affine {
        let turns = degrees.rem_euclid(360.0);
        let (s, c) = if turns == 0.0 {
            (0.0, 1.0)
        } else if turns == 90.0 {
            (1.0, 0.0)
        } else if turns == 180.0 {
            (0.0, -1.0)
        } else if turns == 270.0 {
            (-1.0, 0.0)
        } else {
            turns.to_radians().sin_cos()
        };
        Affine {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    /// A mirror about an axis through the origin.
    pub fn mirror(mirror: Mirror) -> Affine {
        match mirror {
            Mirror::None => Affine::IDENTITY,
            Mirror::X => Affine::scale(-1.0, 1.0),
            Mirror::Y => Affine::scale(1.0, -1.0),
            Mirror::Xy => Affine::scale(-1.0, -1.0),
        }
    }

    /// The exchange of the axes: x' = y and y' = x.
    pub fn swap_axes() -> Affine {
        Affine {
            a: 0.0,
            b: 1.0,
            c: 1.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// The transformation that applies `self` first and then `next`.
    pub fn then(self, next: Affine) -> Affine {
        Affine {
            a: next.a * self.a + next.c * self.b,
            b: next.b * self.a + next.d * self.b,
            c: next.a * self.c + next.c * self.d,
            d: next.b * self.c + next.d * self.d,
            e: next.a * self.e + next.c * self.f + next.e,
            f: next.b * self.e + next.d * self.f + next.f,
        }
    }

    /// A point after the transformation.
    pub fn apply(&self, p: (f64, f64)) -> (f64, f64) {
        (
            self.a * p.0 + self.c * p.1 + self.e,
            self.b * p.0 + self.d * p.1 + self.f,
        )
    }

    /// The transformation without its shift (what happens to a size or a direction).
    pub fn linear(&self) -> Affine {
        Affine {
            e: 0.0,
            f: 0.0,
            ..*self
        }
    }

    /// The determinant of the linear part: negative when the transformation mirrors.
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }
}

/// A piece of the contour of a region.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    /// A straight line to a point.
    Line {
        /// The end.
        to: (f64, f64),
    },
    /// An arc to a point round a centre.
    Arc {
        /// The end.
        to: (f64, f64),
        /// The centre.
        center: (f64, f64),
        /// True when the arc goes clockwise (after the transformations).
        clockwise: bool,
    },
}

impl Segment {
    /// The end of the piece.
    pub fn end(&self) -> (f64, f64) {
        match self {
            Segment::Line { to } | Segment::Arc { to, .. } => *to,
        }
    }
}

/// A closed contour of a region: its start and the pieces that follow. The last piece ends at the start (the
/// reader of a region closes the contour when the file does not).
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    /// Where the contour starts.
    pub start: (f64, f64),
    /// The pieces, in order.
    pub segments: Vec<Segment>,
}

/// What is drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    /// An aperture flashed. `transform` takes the aperture as defined (centred on its origin) to its place.
    Flash {
        /// The number of the aperture (never a block aperture: those are expanded).
        aperture: u32,
        /// Where and how it is placed.
        transform: Affine,
    },
    /// A straight line drawn with the pen of an aperture.
    Stroke {
        /// The number of the aperture.
        aperture: u32,
        /// Start.
        from: (f64, f64),
        /// End.
        to: (f64, f64),
        /// How the pen is turned or stretched (its linear part).
        pen: Affine,
    },
    /// An arc drawn with the pen of an aperture.
    ArcStroke {
        /// The number of the aperture.
        aperture: u32,
        /// Start.
        from: (f64, f64),
        /// End.
        to: (f64, f64),
        /// Centre.
        center: (f64, f64),
        /// True when the arc goes clockwise (after the transformations).
        clockwise: bool,
        /// How the pen is turned or stretched (its linear part).
        pen: Affine,
    },
    /// A filled region, with one or more contours.
    Region {
        /// The contours.
        contours: Vec<Contour>,
    },
}

/// A drawn thing with its polarity and the group it belongs to.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The polarity of the item inside its group (or the layer).
    pub polarity: Polarity,
    /// The group (the index into [`Image::groups`]), or `None` for the layer itself.
    pub group: Option<usize>,
    /// What is drawn.
    pub shape: Shape,
}

/// One flash of a block aperture: its items are composed first.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Group {
    /// The group this one is inside, or `None` when it is in the layer.
    pub parent: Option<usize>,
    /// The polarity with which the finished group is added to what it is inside.
    pub polarity: Polarity,
    /// The number of the block aperture.
    pub aperture: u32,
}

/// The limits of [`resolve_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveLimits {
    /// The most items (a repeat or a block makes many from a few commands).
    pub max_items: usize,
    /// The deepest nesting of block apertures.
    pub max_depth: usize,
}

impl Default for ResolveLimits {
    fn default() -> Self {
        ResolveLimits {
            max_items: 5_000_000,
            max_depth: 16,
        }
    }
}

/// The drawing of a file.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// The units of the coordinates, as the file declares them.
    pub units: Option<Units>,
    /// True when the image is negative (`IPNEG`): the items are the clear part of the layer.
    pub negative: bool,
    /// The items, in the order of the file.
    pub items: Vec<Item>,
    /// The groups made by the flashes of block apertures.
    pub groups: Vec<Group>,
    /// What went wrong while putting the file in place.
    pub diagnostics: Vec<Diagnostic>,
}

impl Image {
    /// The number of flashes, strokes, arc strokes and regions.
    pub fn counts(&self) -> (usize, usize, usize, usize) {
        let mut c = (0, 0, 0, 0);
        for i in &self.items {
            match i.shape {
                Shape::Flash { .. } => c.0 += 1,
                Shape::Stroke { .. } => c.1 += 1,
                Shape::ArcStroke { .. } => c.2 += 1,
                Shape::Region { .. } => c.3 += 1,
            }
        }
        c
    }

    /// The number of strokes and arc strokes drawn with a pen of no width (they are kept: some
    /// manufacturers draw the outline that way).
    pub fn hairlines(&self, file: &GerberFile) -> usize {
        self.items
            .iter()
            .filter(|i| match i.shape {
                Shape::Stroke { aperture, .. } | Shape::ArcStroke { aperture, .. } => {
                    matches!(file.apertures.get(&aperture),
                        Some(Aperture::Circle { diameter, .. }) if *diameter == 0.0)
                        || matches!(file.apertures.get(&aperture),
                        Some(Aperture::Rectangle { x, y, .. }) if *x == 0.0 || *y == 0.0)
                }
                _ => false,
            })
            .count()
    }

    /// The box `[min x, min y, max x, max y]` that holds everything drawn, in the units of the file, or `None`
    /// for an empty image. It is conservative: the box of an aperture that is turned is the box of the turned
    /// box, and a macro is held by the box of its primitives.
    pub fn bounds(&self, file: &GerberFile) -> Option<[f64; 4]> {
        let mut b: Option<[f64; 4]> = None;
        let mut add = |p: (f64, f64)| match &mut b {
            Some(r) => {
                r[0] = r[0].min(p.0);
                r[1] = r[1].min(p.1);
                r[2] = r[2].max(p.0);
                r[3] = r[3].max(p.1);
            }
            None => b = Some([p.0, p.1, p.0, p.1]),
        };
        for item in &self.items {
            match &item.shape {
                Shape::Flash {
                    aperture,
                    transform,
                } => {
                    if let Some(local) = aperture_bounds(file, *aperture) {
                        for p in corners(local) {
                            add(transform.apply(p));
                        }
                    }
                }
                Shape::Stroke {
                    aperture,
                    from,
                    to,
                    pen,
                } => {
                    let pen_corners = pen_extent(file, *aperture, pen);
                    for end in [from, to] {
                        for c in &pen_corners {
                            add((end.0 + c.0, end.1 + c.1));
                        }
                    }
                }
                Shape::ArcStroke {
                    aperture,
                    from,
                    to,
                    center,
                    clockwise,
                    pen,
                } => {
                    let pen_corners = pen_extent(file, *aperture, pen);
                    for p in arc_extremes(*from, *to, *center, *clockwise) {
                        for c in &pen_corners {
                            add((p.0 + c.0, p.1 + c.1));
                        }
                    }
                }
                Shape::Region { contours } => {
                    for contour in contours {
                        add(contour.start);
                        let mut at = contour.start;
                        for s in &contour.segments {
                            match s {
                                Segment::Line { to } => add(*to),
                                Segment::Arc {
                                    to,
                                    center,
                                    clockwise,
                                } => {
                                    for p in arc_extremes(at, *to, *center, *clockwise) {
                                        add(p);
                                    }
                                }
                            }
                            at = s.end();
                        }
                    }
                }
            }
        }
        b
    }
}

fn corners(b: [f64; 4]) -> [(f64, f64); 4] {
    [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])]
}

/// The box of an aperture as defined, centred on its origin, or `None` when it has none.
pub fn aperture_bounds(file: &GerberFile, number: u32) -> Option<[f64; 4]> {
    match file.apertures.get(&number)? {
        Aperture::Circle { diameter, .. } | Aperture::Polygon { diameter, .. } => {
            let r = diameter.abs() / 2.0;
            Some([-r, -r, r, r])
        }
        Aperture::Rectangle { x, y, .. } | Aperture::Obround { x, y, .. } => {
            Some([-x.abs() / 2.0, -y.abs() / 2.0, x.abs() / 2.0, y.abs() / 2.0])
        }
        Aperture::Macro { name, params } => {
            let def = file.macros.get(name)?;
            macros::bounds(&macros::evaluate(def, params))
        }
        Aperture::Block { .. } => None,
    }
}

fn pen_extent(file: &GerberFile, aperture: u32, pen: &Affine) -> Vec<(f64, f64)> {
    match aperture_bounds(file, aperture) {
        Some(local) => corners(local)
            .iter()
            .map(|p| pen.linear().apply(*p))
            .collect(),
        None => vec![(0.0, 0.0)],
    }
}

/// The ends of an arc and the points of its circle on the axes that lie on the arc.
fn arc_extremes(
    from: (f64, f64),
    to: (f64, f64),
    c: (f64, f64),
    clockwise: bool,
) -> Vec<(f64, f64)> {
    let mut out = vec![from, to];
    let r = (from.0 - c.0).hypot(from.1 - c.1);
    let a0 = (from.1 - c.1).atan2(from.0 - c.0);
    let a1 = (to.1 - c.1).atan2(to.0 - c.0);
    let tau = std::f64::consts::TAU;
    let ccw = (a1 - a0).rem_euclid(tau);
    let same = (from.0 - to.0).abs() < 1e-12 && (from.1 - to.1).abs() < 1e-12;
    let sweep = if same {
        tau
    } else if clockwise {
        (tau - ccw).rem_euclid(tau)
    } else {
        ccw
    };
    for k in 0..4 {
        let axis = f64::from(k) * std::f64::consts::FRAC_PI_2;
        let delta = if clockwise {
            (a0 - axis).rem_euclid(tau)
        } else {
            (axis - a0).rem_euclid(tau)
        };
        if delta <= sweep {
            out.push((c.0 + r * axis.cos(), c.1 + r * axis.sin()));
        }
    }
    out
}

/// Puts a file in place with the default limits.
pub fn resolve(file: &GerberFile) -> Image {
    resolve_with(file, &ResolveLimits::default())
}

/// The transformation of the whole image from its parameters (`AS`, `MI`, `SF`, `IR`, `OF`, in that order).
pub fn image_transform(file: &GerberFile) -> Affine {
    let p = &file.image;
    let mut t = Affine::IDENTITY;
    if p.swap_axes {
        t = t.then(Affine::swap_axes());
    }
    t = t.then(Affine::scale(
        if p.mirror_a { -1.0 } else { 1.0 },
        if p.mirror_b { -1.0 } else { 1.0 },
    ));
    t = t.then(Affine::scale(p.scale.0, p.scale.1));
    t = t.then(Affine::rotate_degrees(f64::from(p.rotation)));
    t.then(Affine::translate(p.offset.0, p.offset.1))
}

/// Puts a file in place with the limits given.
pub fn resolve_with(file: &GerberFile, limits: &ResolveLimits) -> Image {
    let mut w = Walker {
        file,
        limits,
        out: Image {
            units: file.units,
            negative: file.image.negative,
            items: Vec::new(),
            groups: Vec::new(),
            diagnostics: Vec::new(),
        },
        stack: Vec::new(),
        stopped: false,
    };
    let frame = Frame {
        base: image_transform(file),
        group: None,
    };
    let mut state = GState::new();
    w.run(&file.commands, &frame, &mut state, (0.0, 0.0));
    w.out
}

struct Walker<'a> {
    file: &'a GerberFile,
    limits: &'a ResolveLimits,
    out: Image,
    /// The block apertures being expanded, the innermost last.
    stack: Vec<u32>,
    stopped: bool,
}

struct Frame {
    base: Affine,
    group: Option<usize>,
}

struct GState {
    aperture: Option<u32>,
    polarity: Polarity,
    mirror: Mirror,
    rotation: f64,
    scale: f64,
    region: Option<RegionBuild>,
}

struct RegionBuild {
    contours: Vec<Contour>,
    current: Option<Contour>,
}

impl GState {
    fn new() -> GState {
        GState {
            aperture: None,
            polarity: Polarity::Dark,
            mirror: Mirror::None,
            rotation: 0.0,
            scale: 1.0,
            region: None,
        }
    }

    fn load_transform(&self) -> Affine {
        Affine::mirror(self.mirror)
            .then(Affine::rotate_degrees(self.rotation))
            .then(Affine::scale(self.scale, self.scale))
    }
}

/// The most notices kept while putting a file in place.
const MAX_NOTICES: usize = 1000;

impl Walker<'_> {
    fn note(&mut self, line: usize, severity: Severity, code: &'static str, message: String) {
        if self.out.diagnostics.len() < MAX_NOTICES {
            self.out
                .diagnostics
                .push(Diagnostic::new(line, severity, code, message));
        }
    }

    fn emit(&mut self, line: usize, polarity: Polarity, group: Option<usize>, shape: Shape) {
        if self.out.items.len() >= self.limits.max_items {
            if !self.stopped {
                self.stopped = true;
                self.note(
                    line,
                    Severity::Error,
                    codes::IMAGE_TOO_LARGE,
                    format!(
                        "more than {} objects: the rest of the drawing is left out",
                        self.limits.max_items
                    ),
                );
            }
            return;
        }
        self.out.items.push(Item {
            polarity,
            group,
            shape,
        });
    }

    fn run(&mut self, cmds: &[Command], frame: &Frame, st: &mut GState, offset: (f64, f64)) {
        let mut i = 0;
        while i < cmds.len() && !self.stopped {
            let cmd = &cmds[i];
            let at = |p: crate::Point| frame.base.apply((p.x + offset.0, p.y + offset.1));
            match &cmd.op {
                Operation::StepRepeat(Some(sr)) => {
                    let mut j = i + 1;
                    while j < cmds.len() && !matches!(cmds[j].op, Operation::StepRepeat(None)) {
                        j += 1;
                    }
                    let inner = &cmds[i + 1..j];
                    'repeat: for iy in 0..sr.y_repeats {
                        for ix in 0..sr.x_repeats {
                            if self.stopped {
                                break 'repeat;
                            }
                            self.run(
                                inner,
                                frame,
                                st,
                                (
                                    offset.0 + f64::from(ix) * sr.x_step,
                                    offset.1 + f64::from(iy) * sr.y_step,
                                ),
                            );
                        }
                    }
                    i = j + 1;
                    continue;
                }
                Operation::StepRepeat(None) => {}
                Operation::SelectAperture(n) => st.aperture = Some(*n),
                Operation::Polarity(p) => st.polarity = *p,
                Operation::LoadMirror(m) => st.mirror = *m,
                Operation::LoadRotation(r) => st.rotation = *r,
                Operation::LoadScale(s) => st.scale = *s,
                Operation::RegionStart => {
                    st.region = Some(RegionBuild {
                        contours: Vec::new(),
                        current: None,
                    });
                }
                Operation::RegionEnd => {
                    if let Some(mut r) = st.region.take() {
                        if let Some(c) = r.current.take() {
                            if !c.segments.is_empty() {
                                r.contours.push(c);
                            }
                        }
                        if !r.contours.is_empty() {
                            self.emit(
                                cmd.line,
                                st.polarity,
                                frame.group,
                                Shape::Region {
                                    contours: r.contours,
                                },
                            );
                        }
                    }
                }
                Operation::Move { to } => {
                    if let Some(r) = st.region.as_mut() {
                        if let Some(c) = r.current.take() {
                            if !c.segments.is_empty() {
                                r.contours.push(c);
                            }
                        }
                        r.current = Some(Contour {
                            start: at(*to),
                            segments: Vec::new(),
                        });
                    }
                }
                Operation::Flash { at: p } => self.flash(cmd.line, *p, frame, st, offset),
                Operation::Line { from, to } => {
                    if let Some(r) = st.region.as_mut() {
                        let c = r.current.get_or_insert_with(|| Contour {
                            start: at(*from),
                            segments: Vec::new(),
                        });
                        c.segments.push(Segment::Line { to: at(*to) });
                    } else if let Some(n) = self.pen(cmd.line, st) {
                        self.emit(
                            cmd.line,
                            st.polarity,
                            frame.group,
                            Shape::Stroke {
                                aperture: n,
                                from: at(*from),
                                to: at(*to),
                                pen: frame.base.linear(),
                            },
                        );
                    }
                }
                Operation::Arc {
                    from,
                    to,
                    center_offset,
                    clockwise,
                    quadrant,
                } => {
                    let Some(g) = arc_geometry(*from, *to, *center_offset, *clockwise, *quadrant)
                    else {
                        self.note(
                            cmd.line,
                            Severity::Error,
                            codes::ARC_CENTRE_MISSING,
                            "an arc with no centre to speak of was left out".to_owned(),
                        );
                        i += 1;
                        continue;
                    };
                    let flipped = frame.base.determinant() < 0.0;
                    let cw = *clockwise != flipped;
                    let center = at(g.center);
                    if let Some(r) = st.region.as_mut() {
                        let c = r.current.get_or_insert_with(|| Contour {
                            start: at(*from),
                            segments: Vec::new(),
                        });
                        c.segments.push(Segment::Arc {
                            to: at(*to),
                            center,
                            clockwise: cw,
                        });
                    } else if let Some(n) = self.pen(cmd.line, st) {
                        self.emit(
                            cmd.line,
                            st.polarity,
                            frame.group,
                            Shape::ArcStroke {
                                aperture: n,
                                from: at(*from),
                                to: at(*to),
                                center,
                                clockwise: cw,
                                pen: frame.base.linear(),
                            },
                        );
                    }
                }
            }
            i += 1;
        }
    }

    /// The aperture a draw uses, or a notice when there is none.
    fn pen(&mut self, line: usize, st: &GState) -> Option<u32> {
        match st.aperture {
            Some(n) if self.file.apertures.contains_key(&n) => {
                if matches!(self.file.apertures.get(&n), Some(Aperture::Block { .. })) {
                    self.note(
                        line,
                        Severity::Error,
                        codes::BLOCK_APERTURE_INVALID,
                        format!("D{n} is a block aperture and cannot draw a line"),
                    );
                    None
                } else {
                    Some(n)
                }
            }
            _ => {
                self.note(
                    line,
                    Severity::Error,
                    codes::APERTURE_UNDEFINED,
                    "a draw with an aperture that is not defined was left out".to_owned(),
                );
                None
            }
        }
    }

    fn flash(
        &mut self,
        line: usize,
        p: crate::Point,
        frame: &Frame,
        st: &GState,
        offset: (f64, f64),
    ) {
        let Some(n) = st.aperture else {
            self.note(
                line,
                Severity::Error,
                codes::APERTURE_NOT_SELECTED,
                "a flash with no aperture selected was left out".to_owned(),
            );
            return;
        };
        let place = st
            .load_transform()
            .then(Affine::translate(p.x + offset.0, p.y + offset.1))
            .then(frame.base);
        match self.file.apertures.get(&n) {
            None => self.note(
                line,
                Severity::Error,
                codes::APERTURE_UNDEFINED,
                format!("a flash of D{n}, which is not defined, was left out"),
            ),
            Some(Aperture::Block { commands }) => {
                if self.stack.contains(&n) {
                    self.note(
                        line,
                        Severity::Error,
                        codes::BLOCK_APERTURE_CYCLE,
                        format!("the block aperture D{n} draws itself and was left out"),
                    );
                    return;
                }
                if self.stack.len() >= self.limits.max_depth {
                    self.note(
                        line,
                        Severity::Error,
                        codes::BLOCK_APERTURE_CYCLE,
                        format!(
                            "block apertures nested deeper than {}: D{n} was left out",
                            self.limits.max_depth
                        ),
                    );
                    return;
                }
                let id = self.out.groups.len();
                self.out.groups.push(Group {
                    parent: frame.group,
                    polarity: st.polarity,
                    aperture: n,
                });
                self.stack.push(n);
                let child = Frame {
                    base: place,
                    group: Some(id),
                };
                let mut child_state = GState::new();
                // `place` already carries the position: inside the block the origin is the block's own
                self.run(commands, &child, &mut child_state, (0.0, 0.0));
                self.stack.pop();
            }
            Some(_) => self.emit(
                line,
                st.polarity,
                frame.group,
                Shape::Flash {
                    aperture: n,
                    transform: place,
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_transformations_compose_in_the_order_they_are_named() {
        let t = Affine::scale(2.0, 2.0)
            .then(Affine::rotate_degrees(90.0))
            .then(Affine::translate(10.0, 0.0));
        let p = t.apply((1.0, 0.0));
        assert_eq!(p, (10.0, 2.0));
        assert!(Affine::mirror(Mirror::X).determinant() < 0.0);
        assert!(Affine::mirror(Mirror::Xy).determinant() > 0.0);
        assert_eq!(Affine::rotate_degrees(450.0).apply((1.0, 0.0)), (0.0, 1.0));
        assert_eq!(Affine::swap_axes().apply((1.0, 2.0)), (2.0, 1.0));
    }
}
