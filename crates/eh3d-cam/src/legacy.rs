//! Old-style Gerber layers (RS-274D): the file has no format statement and no aperture definitions, so the number format, the units and the list of apertures (a separate
//! text table with one D code and one size per line) have to be told. The layer is rewritten as extended Gerber (RS-274X) with those facts at the top, and the normal reader
//! takes it from there.

use eh3d_raster::units::Unit;

/// Which zeros the coordinates leave out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zeros {
    Leading,
    Trailing,
}

impl Zeros {
    pub fn parse(v: &str) -> Result<Zeros, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "leading" | "l" => Ok(Zeros::Leading),
            "trailing" | "t" => Ok(Zeros::Trailing),
            other => Err(format!("zeros '{other}': it must be leading or trailing")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Zeros::Leading => "leading",
            Zeros::Trailing => "trailing",
        }
    }
}

/// The number format of the coordinates: digits before and after the decimal point, and which zeros are left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub int: u8,
    pub dec: u8,
    pub zeros: Zeros,
}

impl Format {
    /// `"2.3"` or `"23"`: digits before and after the decimal point (1 to 6 before, 0 to 6 after).
    pub fn parse_digits(v: &str) -> Result<(u8, u8), String> {
        let t = v.trim();
        let (a, b) = match t.split_once('.') {
            Some((a, b)) => (a.trim().to_string(), b.trim().to_string()),
            None if t.len() == 2 => (t[..1].to_string(), t[1..].to_string()),
            None => {
                return Err(format!(
                    "format '{v}': write it as 2.3 (digits before and after the point)"
                ))
            }
        };
        let (i, d): (u8, u8) = (
            a.parse()
                .map_err(|_| format!("format '{v}': not two numbers"))?,
            b.parse()
                .map_err(|_| format!("format '{v}': not two numbers"))?,
        );
        if !(1..=6).contains(&i) || d > 6 {
            return Err(format!(
                "format '{v}': 1 to 6 digits before and 0 to 6 after the point"
            ));
        }
        Ok((i, d))
    }
}

/// The shape of an aperture of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Round,
    Square,
    Rect,
    Oblong,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Aperture {
    pub code: u32,
    pub shape: Shape,
    pub x_mm: f64,
    pub y_mm: f64,
}

/// The most apertures a list may have (D10 to D999 exist).
pub const MAX_APERTURES: usize = 990;

fn shape_word(w: &str) -> Option<Shape> {
    match w.to_ascii_lowercase().as_str() {
        "circle" | "c" | "round" | "r" | "rd" | "rnd" | "line" | "l" | "rounded" | "donut" => {
            Some(Shape::Round)
        }
        "rectangle" | "rect" | "rec" | "rectangular" => Some(Shape::Rect),
        "square" | "sq" | "sqr" => Some(Shape::Square),
        "oblong" | "obround" | "o" => Some(Shape::Oblong),
        _ => None,
    }
}

fn d_code(token: &str) -> Option<u32> {
    let t = token.trim_matches(|c: char| c == '*' || c == ',' || c == ';');
    let rest = t.strip_prefix('D').or_else(|| t.strip_prefix('d'))?;
    if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let n: u32 = rest.parse().ok()?;
    (10..=9999).contains(&n).then_some(n)
}

/// Reads a list of apertures: every line that holds a D code is one aperture. The size is the `size_column`-th number after the D code (1 is the first; a second number
/// that follows is the other side of a rectangle), in `unit`; a word of the shape names (circle, rect, square, oblong ... any case) tells the shape, and with none it is round.
/// Lines with no D code (headings, frames) are skipped. A size that is not a positive finite number is an error with its line.
pub fn parse_apertures(
    text: &str,
    unit: Unit,
    size_column: usize,
) -> Result<Vec<Aperture>, String> {
    if size_column == 0 {
        return Err("aperture_size_column is 0: it counts from 1".into());
    }
    let mut out: Vec<Aperture> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let tokens: Vec<&str> = line
            .split(|c: char| {
                c.is_whitespace() || c == '\t' || c == '|' || c == ';' || !c.is_ascii()
            })
            .filter(|t| !t.is_empty())
            .collect();
        let Some(pos) = tokens.iter().position(|t| d_code(t).is_some()) else {
            continue;
        };
        let code = d_code(tokens[pos]).unwrap_or(0);
        let mut shape = Shape::Round;
        let mut nums: Vec<f64> = Vec::new();
        for t in &tokens[pos + 1..] {
            let t = t.trim_end_matches(',');
            if let Some(s) = shape_word(t) {
                shape = s;
            } else if let Ok(v) = t.parse::<f64>() {
                nums.push(v);
            } else if let Some((a, b)) = t.split_once(['x', 'X']) {
                // both sides in one cell: 60x40
                if let (Ok(a), Ok(b)) = (a.parse::<f64>(), b.parse::<f64>()) {
                    nums.push(a);
                    nums.push(b);
                }
            }
        }
        let Some(&x) = nums.get(size_column - 1) else {
            return Err(format!(
                "line {}: D{code} has no size in column {size_column}",
                n + 1
            ));
        };
        let y = nums.get(size_column).copied().unwrap_or(x);
        if !(x.is_finite() && x > 0.0 && y.is_finite() && y > 0.0) {
            return Err(format!(
                "line {}: D{code} has a size that is not positive",
                n + 1
            ));
        }
        if out.len() >= MAX_APERTURES {
            return Err(format!("more than {MAX_APERTURES} apertures"));
        }
        if let Some(prev) = out.iter_mut().find(|a| a.code == code) {
            // the last definition of a code wins
            *prev = Aperture {
                code,
                shape,
                x_mm: unit.to_mm(x),
                y_mm: unit.to_mm(y),
            };
        } else {
            out.push(Aperture {
                code,
                shape,
                x_mm: unit.to_mm(x),
                y_mm: unit.to_mm(y),
            });
        }
    }
    if out.is_empty() {
        return Err("the aperture list holds no line with a D code and a size".into());
    }
    Ok(out)
}

/// True when the layer states its own number format (extended Gerber): there is nothing to add.
pub fn has_format_statement(text: &str) -> bool {
    text.contains("%FS")
}

/// The units a layer states by itself: `G70` or `MOIN` is inch, `G71` or `MOMM` is mm; `None` when it says nothing (or says both).
pub fn stated_units(text: &str) -> Option<Unit> {
    let inch = text.contains("G70") || text.contains("MOIN");
    let mm = text.contains("G71") || text.contains("MOMM");
    match (inch, mm) {
        (true, false) => Some(Unit::Inch),
        (false, true) => Some(Unit::Mm),
        _ => None,
    }
}

/// What the coordinates of a file look like: the most digits one has, and how far apart the lowest and the highest value are on each axis (as the bare numbers written).
struct Look {
    longest: usize,
    span_x: f64,
    span_y: f64,
    leading_zero: usize,
    trailing_zero: usize,
}

impl Look {
    fn of(text: &str) -> Option<Look> {
        let b = text.as_bytes();
        let mut look = Look {
            longest: 0,
            span_x: 0.0,
            span_y: 0.0,
            leading_zero: 0,
            trailing_zero: 0,
        };
        let mut range = [(f64::INFINITY, f64::NEG_INFINITY); 2];
        let mut i = 0;
        while i < b.len() {
            if (b[i] == b'X' || b[i] == b'Y') && i + 1 < b.len() {
                let axis = usize::from(b[i] == b'Y');
                let mut j = i + 1;
                let negative = j < b.len() && b[j] == b'-';
                if j < b.len() && (b[j] == b'-' || b[j] == b'+') {
                    j += 1;
                }
                let s = j;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                let digits = j - s;
                if digits > 0 {
                    look.longest = look.longest.max(digits);
                    if b[s] == b'0' {
                        look.leading_zero += 1;
                    }
                    if b[j - 1] == b'0' {
                        look.trailing_zero += 1;
                    }
                    if let Ok(v) = text[s..j.min(s + 15)].parse::<f64>() {
                        let v = if negative { -v } else { v };
                        range[axis].0 = range[axis].0.min(v);
                        range[axis].1 = range[axis].1.max(v);
                    }
                }
                i = j;
            } else {
                i += 1;
            }
        }
        if look.longest == 0 {
            return None;
        }
        let span = |r: (f64, f64)| if r.1 >= r.0 { r.1 - r.0 } else { 0.0 };
        look.span_x = span(range[0]);
        look.span_y = span(range[1]);
        Some(look)
    }

    fn big(&self, limit: f64) -> bool {
        self.span_x > limit || self.span_y > limit
    }
}

/// The digits before and after the point for the number of digits of the longest coordinate, the units (when the file does not state them) and how far the board reaches.
/// A coordinate of five digits is a board of a few inches written 2.3 or a few tens of millimetres written 3.2; the size of the board tells them apart.
fn digits_for(look: &Look, units: Option<Unit>) -> (Unit, u8, u8) {
    use Unit::{Inch, Mm};
    match (look.longest.min(8), units) {
        (8, Some(Mm)) => (Mm, 4, 4),
        (8, Some(_)) => {
            if look.big(200_000.0) {
                (Inch, 4, 4)
            } else {
                (Inch, 3, 5)
            }
        }
        (8, None) => {
            if look.big(10_000_000.0) {
                (Inch, 3, 5)
            } else {
                (Mm, 4, 4)
            }
        }
        (7, Some(Mm)) => (Mm, 3, 4),
        (7, Some(_)) => (Inch, 2, if look.big(1_500_000.0) { 6 } else { 5 }),
        (7, None) => {
            if look.big(800_000.0) {
                (Inch, 2, 5)
            } else {
                (Mm, 3, 4)
            }
        }
        (6, Some(Mm)) => {
            if look.big(40_000.0) && !look.big(400_000.0) {
                (Mm, 3, 3)
            } else {
                (Mm, 4, 2)
            }
        }
        (6, Some(_)) => (Inch, 2, if look.big(150_000.0) { 5 } else { 4 }),
        (6, None) => {
            if !look.big(150_000.0) {
                (Inch, 2, 4)
            } else if look.big(800_000.0) {
                (Mm, 2, 4)
            } else {
                (Mm, 3, 3)
            }
        }
        (5, Some(Mm)) => (Mm, 3, if look.big(40_000.0) { 2 } else { 3 }),
        (5, Some(_)) => {
            if look.big(25_000.0) && look.big(40_000.0) {
                (Inch, 1, 4)
            } else {
                (Inch, 2, 3)
            }
        }
        (5, None) => {
            if !look.big(15_000.0) {
                (Inch, 2, 3)
            } else if look.big(40_000.0) {
                (Inch, 1, 4)
            } else {
                (Mm, 3, 2)
            }
        }
        (_, Some(Mm)) => (Mm, 3, 3),
        _ => (Inch, 2, 3),
    }
}

/// A guess of the format and of the units from the longest coordinate of the file and the size of the board it draws. `None` when the file holds no coordinate.
/// It is a hint only: the digits can look alike (2.3 and 2.4 with few zeros), and the person confirms it. The units the file states win over the guess.
pub fn guess_format_and_units(text: &str) -> Option<(Format, Unit)> {
    let look = Look::of(text)?;
    // a file that keeps its leading zeros and drops the trailing ones has many numbers that start with 0 and few that end with it; the reverse is a file that drops the leading ones
    let zeros = if look.leading_zero > look.trailing_zero {
        Zeros::Trailing
    } else {
        Zeros::Leading
    };
    let (unit, int, dec) = digits_for(&look, stated_units(text));
    Some((Format { int, dec, zeros }, unit))
}

/// The format alone of `guess_format_and_units`.
pub fn guess_format(text: &str) -> Option<Format> {
    guess_format_and_units(text).map(|(f, _)| f)
}

fn mm_text(v: f64) -> String {
    let t = format!("{v:.6}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t.is_empty() {
        "0".into()
    } else {
        t.into()
    }
}

/// The layer as extended Gerber: the format, the units and the apertures first, then the old statements one by one (the preparatory codes that old files carry before a
/// D code, `G54`, go with the D code the reader already understands). `units` is used when the file says nothing (`G70` is inch, `G71` is mm).
pub fn to_extended(
    text: &str,
    fmt: Format,
    units: Unit,
    apertures: &[Aperture],
) -> Result<String, String> {
    if !matches!(units, Unit::Mm | Unit::Inch) {
        return Err("the units of a Gerber layer are mm or inch".into());
    }
    if has_format_statement(text) {
        return Err("the layer already has a format statement: it is read as it is".into());
    }
    let stated = if text.contains("G70") {
        Some(Unit::Inch)
    } else if text.contains("G71") {
        Some(Unit::Mm)
    } else {
        None
    };
    let u = stated.unwrap_or(units);
    let z = match fmt.zeros {
        Zeros::Leading => 'L',
        Zeros::Trailing => 'T',
    };
    let mut out = format!(
        "%FS{z}AX{i}{d}Y{i}{d}*%\n%MO{}*%\n",
        if u == Unit::Mm { "MM" } else { "IN" },
        i = fmt.int,
        d = fmt.dec
    );
    for a in apertures {
        // the sizes are kept in the unit of the file
        let (x, y) = (u.from_mm(a.x_mm), u.from_mm(a.y_mm));
        let def = match a.shape {
            Shape::Round => format!("C,{}", mm_text(x)),
            Shape::Square => format!("R,{}X{}", mm_text(x), mm_text(x)),
            Shape::Rect => format!("R,{}X{}", mm_text(x), mm_text(y)),
            Shape::Oblong => format!("O,{}X{}", mm_text(x), mm_text(y)),
        };
        out.push_str(&format!("%ADD{}{}*%\n", a.code, def));
    }
    let mut closed = false;
    for stmt in text.split('*') {
        let s: String = stmt.chars().filter(|c| !c.is_whitespace()).collect();
        if s.is_empty() {
            continue;
        }
        if s.starts_with("M02") || s.starts_with("M00") {
            closed = true;
        }
        // G70/G71/G90 were used above or are the default
        if s == "G70" || s == "G71" || s == "G90" {
            continue;
        }
        let s = s
            .strip_prefix("G54")
            .filter(|r| !r.is_empty())
            .unwrap_or(&s);
        out.push_str(s);
        out.push_str("*\n");
    }
    if !closed {
        out.push_str("M02*\n");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_digits() {
        assert_eq!(Format::parse_digits("2.3"), Ok((2, 3)));
        assert_eq!(Format::parse_digits("24"), Ok((2, 4)));
        assert!(Format::parse_digits("0.3").is_err());
        assert!(Format::parse_digits("7.3").is_err());
        assert!(Format::parse_digits("x").is_err());
    }

    #[test]
    fn list_of_apertures() {
        let t = "heading\n  aperture  mils  mm\n D10*  3  0.08\n D11*  4  0.10\n D20 sq 60\n D21 rect 60 40\n D22 oblong 50 30\n";
        let a = parse_apertures(t, Unit::Mil, 1).unwrap();
        assert_eq!(a.len(), 5);
        assert_eq!(a[0].code, 10);
        assert!((a[0].x_mm - 0.0762).abs() < 1e-9);
        assert_eq!(a[2].shape, Shape::Square);
        assert_eq!(a[3].shape, Shape::Rect);
        assert!((a[3].y_mm - 1.016).abs() < 1e-9);
        assert_eq!(a[4].shape, Shape::Oblong);
    }

    #[test]
    fn list_second_column_and_errors() {
        let a = parse_apertures("D10 3 0.08\n", Unit::Mm, 2).unwrap();
        assert!((a[0].x_mm - 0.08).abs() < 1e-9);
        assert!(parse_apertures("D10 0\n", Unit::Mm, 1).is_err());
        assert!(parse_apertures("D10\n", Unit::Mm, 1).is_err());
        assert!(parse_apertures("nothing here\n", Unit::Mm, 1).is_err());
        assert!(parse_apertures("D10 3\n", Unit::Mm, 0).is_err());
        assert!(parse_apertures("D10 -3\n", Unit::Mm, 1).is_err());
    }

    #[test]
    fn last_definition_of_a_code_wins() {
        let a = parse_apertures("D10 3\nD10 5\n", Unit::Mil, 1).unwrap();
        assert_eq!(a.len(), 1);
        assert!((a[0].x_mm - 0.127).abs() < 1e-9);
    }

    #[test]
    fn rewrite_as_extended() {
        let old = "G90*\nG70*\nG54D10*\nX1000Y2000D02*\nX3000Y2000D01*\n";
        let ap = parse_apertures("D10 10\n", Unit::Mil, 1).unwrap();
        let x = to_extended(
            old,
            Format {
                int: 2,
                dec: 3,
                zeros: Zeros::Leading,
            },
            Unit::Mm,
            &ap,
        )
        .unwrap();
        assert!(x.starts_with("%FSLAX23Y23*%\n%MOIN*%\n%ADD10C,0.01*%\n"));
        assert!(x.contains("D10*\nX1000Y2000D02*"));
        assert!(!x.contains("G70"));
        assert!(x.ends_with("M02*\n"));
    }

    #[test]
    fn extended_file_is_left_alone() {
        assert!(to_extended(
            "%FSLAX24Y24*%\n",
            Format {
                int: 2,
                dec: 4,
                zeros: Zeros::Leading
            },
            Unit::Mm,
            &[]
        )
        .is_err());
        assert!(has_format_statement("%FSLAX24Y24*%"));
        assert!(!has_format_statement("G90*\n"));
    }

    #[test]
    fn units_from_the_file_win() {
        let x = to_extended(
            "G71*\nD10*\n",
            Format {
                int: 3,
                dec: 3,
                zeros: Zeros::Leading,
            },
            Unit::Inch,
            &[],
        )
        .unwrap();
        assert!(x.contains("%MOMM*%"));
        assert!(to_extended(
            "D10*\n",
            Format {
                int: 3,
                dec: 3,
                zeros: Zeros::Leading
            },
            Unit::Mil,
            &[]
        )
        .is_err());
    }

    #[test]
    fn guess_looks_at_the_longest_number() {
        let f = guess_format("X123456Y1000D01*\nX-200Y2000D01*\n").unwrap();
        assert_eq!(f.int + f.dec, 6);
        assert!(guess_format("G90*\nM02*\n").is_none());
    }

    #[test]
    fn short_coordinates_in_inch_are_two_three() {
        // thousandths of an inch: X7371 is 7.371 in
        let t = "G90*\nG70*\nG54D20*\nX631Y4603D02*\nX7371Y5626D01*\nX-24Y3984D01*\n";
        let (f, u) = guess_format_and_units(t).unwrap();
        assert_eq!((f.int, f.dec, u), (2, 3, Unit::Inch));
        assert_eq!(f.zeros, Zeros::Leading);
    }

    #[test]
    fn the_size_of_the_board_tells_the_digits_apart() {
        // five digits and stated mm: 3.3 for a small board, 3.2 for a big one
        let (f, u) = guess_format_and_units("G71*\nX12345Y23456D01*\nX100Y200D02*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (3, 3, Unit::Mm));
        let (f, u) = guess_format_and_units("G71*\nX45000Y23456D01*\nX100Y200D02*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (3, 2, Unit::Mm));
        // five digits with no units: a small span is inch 2.3, a long one inch 1.4, a middle one mm 3.2
        let (f, u) = guess_format_and_units("X10000Y10000D01*\nX100Y100D02*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (2, 3, Unit::Inch));
        let (f, u) = guess_format_and_units("X50000Y10000D01*\nX100Y100D02*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (1, 4, Unit::Inch));
        let (f, u) = guess_format_and_units("X30000Y10000D01*\nX100Y100D02*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (3, 2, Unit::Mm));
        // seven digits: the units the file states win
        let (f, u) = guess_format_and_units("G71*\nX1234567Y2000000D01*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (3, 4, Unit::Mm));
        let (f, u) = guess_format_and_units("G70*\nX0100000Y0D02*\nX2000000Y1D01*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (2, 6, Unit::Inch));
        let (f, u) = guess_format_and_units("G70*\nX1234567Y2000000D01*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (2, 5, Unit::Inch));
        // eight digits
        let (f, u) = guess_format_and_units("G71*\nX12345678Y1D01*\n").unwrap();
        assert_eq!((f.int, f.dec, u), (4, 4, Unit::Mm));
    }

    #[test]
    fn stated_units_are_read() {
        assert_eq!(stated_units("%MOIN*%"), Some(Unit::Inch));
        assert_eq!(stated_units("G71*"), Some(Unit::Mm));
        assert_eq!(stated_units("G70*G71*"), None);
        assert_eq!(stated_units("D10*"), None);
    }

    #[test]
    fn both_sides_in_one_cell() {
        let a = parse_apertures("D20 rect 60x40\nD21 70X70\n", Unit::Mil, 1).unwrap();
        assert!((a[0].x_mm - 1.524).abs() < 1e-9);
        assert!((a[0].y_mm - 1.016).abs() < 1e-9);
        assert!((a[1].x_mm - 1.778).abs() < 1e-9);
        assert!((a[1].y_mm - 1.778).abs() < 1e-9);
    }

    #[test]
    fn garbage_does_not_panic() {
        for t in [
            "",
            "\u{0}\u{1}*",
            "D10",
            "XYXY",
            "X-Y-",
            "%%%",
            "D999999999*",
        ] {
            let _ = parse_apertures(t, Unit::Mm, 1);
            let _ = guess_format(t);
            let _ = to_extended(
                t,
                Format {
                    int: 2,
                    dec: 3,
                    zeros: Zeros::Trailing,
                },
                Unit::Mm,
                &[],
            );
        }
    }
}
