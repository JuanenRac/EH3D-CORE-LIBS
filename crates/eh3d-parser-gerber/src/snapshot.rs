//! The normalised text of a reading, to compare one reading with another.
//!
//! [`normalize`] writes what a reader understood of a file as plain text that does not change from one run
//! to the next, from one machine to the next, or when the order of a map changes: the dialect, the units and
//! the format, the apertures one by one, the macros, the count of every kind of command, the drawing put in
//! place by [`crate::image::resolve`] (how many flashes, strokes, arcs, regions and groups, and the box that holds
//! them) and every diagnostic with its line, severity and code.
//!
//! Every length is written as a whole number of millionths of the unit of the file, so no number is written
//! with a decimal point and no rounding of the machine can show up. The tests of the corpus keep this text
//! next to each synthetic file and fail when a change of the reader changes it; they are the safety net of
//! every change of the reader, and of the move of the other readers of the suite to this one.

use crate::image::{resolve, Shape};
use crate::{Aperture, GerberFile, Operation, Severity};
use std::fmt::Write;

/// A length as a whole number of millionths of its unit.
pub fn micro(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1e6).round().clamp(-9.0e18, 9.0e18) as i64
}

fn units_name(file: &GerberFile) -> &'static str {
    match file.units {
        Some(crate::Units::Millimetres) => "mm",
        Some(crate::Units::Inches) => "in",
        None => "none",
    }
}

/// The normalised text of a reading. See the module documentation.
pub fn normalize(file: &GerberFile) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "dialect: {:?}", file.dialect);
    let _ = writeln!(out, "units: {}", units_name(file));
    match file.format {
        Some(f) => {
            let _ = writeln!(
                out,
                "format: {:?} {}:{}",
                f.zero_omission, f.integer_digits, f.decimal_digits
            );
        }
        None => {
            let _ = writeln!(out, "format: none");
        }
    }
    let _ = writeln!(out, "ended: {}", file.ended);
    let p = &file.image;
    if p.negative || !p.is_identity() || p.name.is_some() {
        let _ = writeln!(
            out,
            "image: negative={} offset=({},{}) scale=({},{}) mirror=({},{}) rotation={} swap={} name={}",
            p.negative,
            micro(p.offset.0),
            micro(p.offset.1),
            micro(p.scale.0),
            micro(p.scale.1),
            p.mirror_a,
            p.mirror_b,
            p.rotation,
            p.swap_axes,
            p.name.as_deref().unwrap_or("")
        );
    }
    let _ = writeln!(out, "attributes: {}", file.attributes.len());
    for a in &file.attributes {
        let _ = writeln!(out, "  {}{} {}", a.kind, a.name, a.values.join(","));
    }
    let _ = writeln!(out, "apertures: {}", file.apertures.len());
    for (n, a) in &file.apertures {
        let _ = match a {
            Aperture::Circle { diameter, hole } => writeln!(
                out,
                "  D{n} circle d={} hole={}",
                micro(*diameter),
                hole.map_or(-1, micro)
            ),
            Aperture::Rectangle { x, y, hole } => writeln!(
                out,
                "  D{n} rectangle x={} y={} hole={}",
                micro(*x),
                micro(*y),
                hole.map_or(-1, micro)
            ),
            Aperture::Obround { x, y, hole } => writeln!(
                out,
                "  D{n} obround x={} y={} hole={}",
                micro(*x),
                micro(*y),
                hole.map_or(-1, micro)
            ),
            Aperture::Polygon {
                diameter,
                vertices,
                rotation,
                hole,
            } => writeln!(
                out,
                "  D{n} polygon d={} n={} rot={} hole={}",
                micro(*diameter),
                vertices,
                micro(*rotation),
                hole.map_or(-1, micro)
            ),
            Aperture::Macro { name, params } => writeln!(
                out,
                "  D{n} macro {name} params=[{}]",
                params
                    .iter()
                    .map(|v| micro(*v).to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Aperture::Block { commands } => {
                writeln!(out, "  D{n} block commands={}", commands.len())
            }
        };
    }
    let _ = writeln!(out, "macros: {}", file.macros.len());
    for (name, m) in &file.macros {
        let _ = writeln!(out, "  {name} lines={}", m.body.len());
    }
    let mut counts = [0usize; 12];
    for c in &file.commands {
        let slot = match c.op {
            Operation::SelectAperture(_) => 0,
            Operation::Move { .. } => 1,
            Operation::Flash { .. } => 2,
            Operation::Line { .. } => 3,
            Operation::Arc { .. } => 4,
            Operation::RegionStart => 5,
            Operation::RegionEnd => 6,
            Operation::Polarity(_) => 7,
            Operation::StepRepeat(_) => 8,
            Operation::LoadMirror(_) => 9,
            Operation::LoadRotation(_) => 10,
            Operation::LoadScale(_) => 11,
        };
        counts[slot] += 1;
    }
    let _ = writeln!(
        out,
        "commands: select={} move={} flash={} line={} arc={} region_start={} region_end={} polarity={} step_repeat={} mirror={} rotation={} scale={}",
        counts[0], counts[1], counts[2], counts[3], counts[4], counts[5], counts[6], counts[7], counts[8], counts[9], counts[10], counts[11]
    );
    let image = resolve(file);
    let (flashes, strokes, arcs, regions) = image.counts();
    let _ = writeln!(
        out,
        "drawing: flashes={flashes} strokes={strokes} arcs={arcs} regions={regions} groups={} hairlines={}",
        image.groups.len(),
        image.hairlines(file)
    );
    let contours: usize = image
        .items
        .iter()
        .map(|i| match &i.shape {
            Shape::Region { contours } => contours.len(),
            _ => 0,
        })
        .sum();
    let _ = writeln!(out, "contours: {contours}");
    match image.bounds(file) {
        Some(b) => {
            let _ = writeln!(
                out,
                "bounds: {} {} {} {}",
                micro(b[0]),
                micro(b[1]),
                micro(b[2]),
                micro(b[3])
            );
        }
        None => {
            let _ = writeln!(out, "bounds: none");
        }
    }
    let _ = writeln!(out, "diagnostics: {}", file.diagnostics.len());
    for d in file.diagnostics.iter().chain(image.diagnostics.iter()) {
        let sev = match d.severity {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        let _ = writeln!(out, "  line {} {sev} {}", d.line, d.code);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_whole_millionths() {
        assert_eq!(micro(1.5), 1_500_000);
        assert_eq!(micro(-0.000_000_4), 0);
        assert_eq!(micro(f64::NAN), 0);
        assert_eq!(micro(1e300), 9_000_000_000_000_000_000);
    }

    #[test]
    fn the_text_is_the_same_every_time_and_says_what_was_read() {
        let text = "%FSLAX24Y24*%\n%MOIN*%\n%ADD10C,0.01*%\nD10*\nX0Y0D02*\nX10000Y0D01*\nX5000Y5000D03*\nM02*\n";
        let a = normalize(&crate::parse(text));
        let b = normalize(&crate::parse(text));
        assert_eq!(a, b);
        assert!(a.contains("units: in"));
        assert!(a.contains("D10 circle d=10000 hole=-1"));
        assert!(a.contains("drawing: flashes=1 strokes=1"));
        assert!(a.contains("bounds: -5000 -5000 1005000 505000"), "{a}");
        assert!(!a.contains('.'), "no decimal point in the text:\n{a}");
    }
}
