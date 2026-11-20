//! Properties of the Gerber reader checked on inputs made from a generator with a fixed seed: the coordinates of every
//! format are read back without loss, the arithmetic of the macros agrees with the value it was made from, the
//! copies of a repeat are as many as the repeat says, the damage of a file never breaks the reader and every
//! diagnostic points at a line that exists.

use eh3d_parser_gerber::image::resolve;
use eh3d_parser_gerber::macros::{eval_expression, evaluate};
use eh3d_parser_gerber::snapshot::normalize;
use eh3d_parser_gerber::{
    decode_coordinate, parse, CoordinateFormat, MacroDefinition, ZeroOmission,
};
use std::collections::BTreeMap;

/// A small deterministic generator (SplitMix64): the same seed always gives the same numbers.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The digits a format writes for a whole number of its smallest unit, with the zeros left out as the format says.
fn encode(units: i64, integer: u32, decimal: u32, zeros: ZeroOmission) -> String {
    let width = (integer + decimal) as usize;
    let sign = if units < 0 { "-" } else { "" };
    let full = format!("{:0>width$}", units.unsigned_abs());
    match zeros {
        ZeroOmission::Leading => {
            let t = full.trim_start_matches('0');
            format!("{sign}{}", if t.is_empty() { "0" } else { t })
        }
        ZeroOmission::Trailing => {
            let t = full.trim_end_matches('0');
            format!("{sign}{}", if t.is_empty() { "0" } else { t })
        }
    }
}

/// Decoding what was encoded gives back the number, for every format and both ways of leaving zeros out.
#[test]
fn a_coordinate_is_read_back_without_loss_in_every_format() {
    let mut rng = Seeded(0xC0DE);
    for integer in 1..=6u32 {
        for decimal in 0..=6u32 {
            for zeros in [ZeroOmission::Leading, ZeroOmission::Trailing] {
                let format = CoordinateFormat {
                    zero_omission: zeros,
                    integer_digits: integer,
                    decimal_digits: decimal,
                };
                let max = 10i64.pow(integer + decimal) - 1;
                for _ in 0..40 {
                    let units =
                        rng.below(max as u64 + 1) as i64 * if rng.below(2) == 0 { 1 } else { -1 };
                    let text = encode(units, integer, decimal, zeros);
                    let value = decode_coordinate(&text, format)
                        .unwrap_or_else(|| panic!("{text} in {integer}.{decimal}"));
                    let back = (value * 10f64.powi(decimal as i32)).round() as i64;
                    assert_eq!(back, units, "{text} in {integer}.{decimal} {zeros:?}");
                }
                // one digit more than the format holds is refused, not truncated
                let too_long = "9".repeat((integer + decimal) as usize + 1);
                assert_eq!(decode_coordinate(&too_long, format), None);
            }
        }
    }
}

/// A random expression with the value it must have.
fn expression(rng: &mut Seeded, depth: u32) -> (String, f64) {
    if depth == 0 || rng.below(4) == 0 {
        let v = rng.below(20) as f64 / 2.0 + 0.5;
        return (format!("{v}"), v);
    }
    let (a, va) = expression(rng, depth - 1);
    let (b, vb) = expression(rng, depth - 1);
    match rng.below(6) {
        0 => (format!("({a}+{b})"), va + vb),
        1 => (format!("({a}-{b})"), va - vb),
        2 => (format!("{a}x{b}"), va * vb),
        3 => (format!("({a})X({b})"), va * vb),
        4 => (format!("-({a}+{b})"), -(va + vb)),
        _ => (format!("({a})/({b})"), va / vb),
    }
}

/// The arithmetic of a macro agrees with the value the expression was built from (the grammar puts a product and a
/// quotient before a sum, as the standard says, and a unary minus on what follows it).
#[test]
fn the_arithmetic_of_a_macro_gives_the_value_it_was_built_from() {
    let mut rng = Seeded(77);
    let none: BTreeMap<u32, f64> = BTreeMap::new();
    for _ in 0..500 {
        let (text, want) = expression(&mut rng, 4);
        match eval_expression(&text, &none) {
            Some(got) => assert!(
                (got - want).abs() <= 1e-9 * want.abs().max(1.0),
                "{text}: {got} against {want}"
            ),
            None => assert!(
                !want.is_finite() || text.contains('/'),
                "{text} should be readable"
            ),
        }
    }
    // the variables take the value of the aperture
    let mut vars = BTreeMap::new();
    vars.insert(1, 4.0);
    vars.insert(2, 0.5);
    assert_eq!(eval_expression("$1x$2+$3", &vars), Some(2.0));
    assert_eq!(eval_expression("-$1+(3-1)/$2", &vars), Some(0.0));
}

/// A step and repeat of `x` by `y` copies of a block of `k` flashes makes `x * y * k` flashes, each in its place.
#[test]
fn the_copies_of_a_repeat_are_as_many_as_the_repeat_says() {
    let mut rng = Seeded(5150);
    for round in 0..60 {
        let (x, y, k) = (1 + rng.below(6), 1 + rng.below(6), 1 + rng.below(5));
        let mut text = String::from("%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\n");
        text.push_str(&format!("%SRX{x}Y{y}I3.0J4.0*%\n"));
        for i in 0..k {
            text.push_str(&format!("X{}Y0D03*\n", i * 500_000));
        }
        text.push_str("%SR*%\nM02*\n");
        let file = parse(&text);
        assert!(file.is_clean(), "round {round}: {:?}", file.diagnostics);
        let image = resolve(&file);
        assert_eq!(image.items.len() as u64, x * y * k, "round {round}");
        // every position appears once
        let mut at: Vec<(i64, i64)> = image
            .items
            .iter()
            .map(|i| match &i.shape {
                eh3d_parser_gerber::image::Shape::Flash { transform, .. } => {
                    let p = transform.apply((0.0, 0.0));
                    ((p.0 * 1e6).round() as i64, (p.1 * 1e6).round() as i64)
                }
                other => panic!("{other:?}"),
            })
            .collect();
        at.sort_unstable();
        let before = at.len();
        at.dedup();
        assert_eq!(
            at.len(),
            before,
            "round {round}: two copies in the same place"
        );
    }
}

/// Damage to a valid file never breaks the reader, the resolver or the snapshot, and every notice points at a line
/// that exists.
#[test]
fn damaged_files_never_break_the_reader_and_every_notice_has_a_line_that_exists() {
    let sources = [
        "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.5*%\n%ADD11R,1X1*%\n%ABD12*%\nD10*\nX0Y0D02*\nX5000000Y0D01*\n%AB*%\n%SRX2Y2I5.0J5.0*%\nD12*\nX1000000Y1000000D03*\n%SR*%\n%LMX*%\n%LR30*%\nD11*\nX9000000Y9000000D03*\nG36*\nX0Y0D02*\nG75*\nG03*\nX1000000Y1000000I500000J0D01*\nX0Y0D01*\nG37*\nM02*\n",
        "%FSLIX24Y24*%\n%MOIN*%\n%IPNEG*%\n%OFA1B1*%\n%AMM*\n$1=2*\n21,1,$1,1,0,0,0*%\n%ADD10M,3*%\nD10*\nX10000Y10000D03*\nG91*\nX5000Y0D01*\nM02*\n",
    ];
    let mut rng = Seeded(0xBAD);
    for source in sources {
        let bytes = source.as_bytes();
        for round in 0..400 {
            let mut copy = bytes.to_vec();
            match rng.below(4) {
                0 => {
                    let cut = rng.below(copy.len() as u64 + 1) as usize;
                    copy.truncate(cut);
                }
                1 => {
                    for _ in 0..1 + rng.below(4) {
                        let i = rng.below(copy.len() as u64) as usize;
                        copy[i] = rng.below(256) as u8;
                    }
                }
                2 => {
                    let i = rng.below(copy.len() as u64) as usize;
                    let j = rng.below(copy.len() as u64) as usize;
                    copy.swap(i, j);
                }
                _ => {
                    let i = rng.below(copy.len() as u64) as usize;
                    let piece: Vec<u8> = copy[i..(i + 10).min(copy.len())].to_vec();
                    copy.splice(i..i, piece);
                }
            }
            let text = String::from_utf8_lossy(&copy).into_owned();
            let file = parse(&text);
            let lines = text.lines().count() + 2;
            for d in &file.diagnostics {
                assert!(
                    d.line >= 1 && d.line <= lines,
                    "round {round}: line {} of {lines}",
                    d.line
                );
            }
            let image = resolve(&file);
            for d in &image.diagnostics {
                assert!(
                    d.line >= 1 && d.line <= lines,
                    "round {round}: line {} of {lines}",
                    d.line
                );
            }
            let snapshot = normalize(&file);
            assert_eq!(
                snapshot,
                normalize(&parse(&text)),
                "round {round}: the reading is not deterministic"
            );
        }
    }
}

/// A macro made of random primitives with random values is evaluated without a panic and every primitive that is kept
/// has a box.
#[test]
fn random_macros_are_evaluated_without_a_panic() {
    let mut rng = Seeded(31);
    for _ in 0..300 {
        let mut body = Vec::new();
        for _ in 0..1 + rng.below(6) {
            let code = [1, 2, 4, 5, 6, 7, 20, 21, 22, 0, 99][rng.below(11) as usize];
            let fields: Vec<String> = (0..rng.below(11))
                .map(|_| format!("{}", rng.below(9) as i64 - 3))
                .collect();
            body.push(format!("{code},{}", fields.join(",")));
        }
        let def = MacroDefinition {
            name: "R".into(),
            body,
        };
        let e = evaluate(&def, &[1.0, 2.0]);
        for p in &e.primitives {
            let _ = p.bounds();
            let _ = p.corners();
        }
        assert!(e.skipped.iter().all(|s| s.0 < def.body.len()));
    }
}
