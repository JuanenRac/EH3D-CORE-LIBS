//! Units of length: millimetres, mils (thousandths of an inch) and inches. A length in a settings file or on the command line may carry its unit (`8mil`, `0.01in`,
//! `0.2mm`); a bare number is in the unit chosen by `input_units`. G-code can be written in millimetres or in inches.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Mm,
    Mil,
    Inch,
}

impl Unit {
    pub fn parse(v: &str) -> Result<Unit, String> {
        match v.trim().to_ascii_lowercase().as_str() {
            "mm" | "millimetre" | "millimetres" | "millimeter" | "millimeters" => Ok(Unit::Mm),
            "mil" | "mils" | "thou" => Ok(Unit::Mil),
            "in" | "inch" | "inches" | "\"" => Ok(Unit::Inch),
            other => Err(format!("unit '{other}': it must be mm, mil or inch")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Unit::Mm => "mm",
            Unit::Mil => "mil",
            Unit::Inch => "inch",
        }
    }

    /// How many millimetres one of this unit is.
    pub fn mm(self) -> f64 {
        match self {
            Unit::Mm => 1.0,
            Unit::Mil => 0.0254,
            Unit::Inch => 25.4,
        }
    }

    pub fn to_mm(self, v: f64) -> f64 {
        v * self.mm()
    }

    pub fn from_mm(self, v: f64) -> f64 {
        v / self.mm()
    }
}

/// A length with or without its unit, in millimetres. A number with no unit is in `bare`. The error says what was wrong.
pub fn parse_length(text: &str, bare: Unit) -> Result<f64, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("a length is empty".into());
    }
    let lower = t.to_ascii_lowercase();
    // the longest suffix first, so that "mils" is not read as "s" after "mil"
    const SUFFIXES: [(&str, Unit); 10] = [
        ("millimetres", Unit::Mm),
        ("millimeters", Unit::Mm),
        ("inches", Unit::Inch),
        ("inch", Unit::Inch),
        ("mils", Unit::Mil),
        ("mil", Unit::Mil),
        ("thou", Unit::Mil),
        ("mm", Unit::Mm),
        ("in", Unit::Inch),
        ("\"", Unit::Inch),
    ];
    let (number, unit) = SUFFIXES
        .iter()
        .find_map(|(s, u)| lower.strip_suffix(s).map(|n| (n, *u)))
        .unwrap_or((lower.as_str(), bare));
    let v: f64 = number.trim().parse().map_err(|_| {
        format!("'{text}' is not a length (a number with an optional unit: mm, mil or in)")
    })?;
    if !v.is_finite() {
        return Err(format!("'{text}' is not a number"));
    }
    Ok(unit.to_mm(v))
}
