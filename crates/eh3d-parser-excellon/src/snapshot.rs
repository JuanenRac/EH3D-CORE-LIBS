//! The normalised text of a reading of a drill file, to compare one reading with another.
//!
//! [`normalize`] writes what the reader understood as plain text that is the same on every run and every machine:
//! the dialect, the units, the zeros, the attributes, the tools one by one, the notes of the CAD program, the number
//! and the box of the holes, the slots, the routes with their arcs and every diagnostic with its line, severity and
//! code. Every length is a whole number of millionths of the unit of the file.

use crate::{ExcellonFile, Severity};
use std::fmt::Write;

/// A length as a whole number of millionths of its unit.
pub fn micro(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1e6).round().clamp(-9.0e18, 9.0e18) as i64
}

/// The normalised text of a reading. See the module documentation.
pub fn normalize(file: &ExcellonFile) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "dialect: {:?}", file.dialect);
    let _ = writeln!(
        out,
        "units: {}",
        match file.units {
            Some(crate::Units::Millimetres) => "mm",
            Some(crate::Units::Inches) => "in",
            None => "none",
        }
    );
    let _ = writeln!(out, "zeros: {:?}", file.zeros);
    let _ = writeln!(out, "plating: {:?}", file.plating());
    let _ = writeln!(out, "ended: {}", file.ended);
    let _ = writeln!(out, "attributes: {}", file.attributes.len());
    for a in &file.attributes {
        let _ = writeln!(out, "  {}{} {}", a.kind, a.name, a.values.join(","));
    }
    let _ = writeln!(out, "tools: {}", file.tools.len());
    for (n, t) in &file.tools {
        let used = file.drills.iter().filter(|h| h.tool == *n).count();
        let _ = writeln!(
            out,
            "  T{n} d={} holes={used} feed={} speed={}",
            micro(t.diameter),
            t.feed.map_or(-1, micro),
            t.speed.map_or(-1, micro)
        );
    }
    let _ = writeln!(out, "notes: {}", file.notes.len());
    for (n, note) in &file.notes {
        let _ = writeln!(
            out,
            "  T{n} plated={:?} quantity={:?} tolerance=+{}/-{}",
            note.plated,
            note.quantity,
            note.tolerance_plus.map_or(-1, micro),
            note.tolerance_minus.map_or(-1, micro)
        );
    }
    let _ = writeln!(out, "holes: {}", file.drills.len());
    if file.drills.is_empty() {
        let _ = writeln!(out, "box: none");
    } else {
        let xs = file.drills.iter().map(|h| micro(h.at.x));
        let ys = file.drills.iter().map(|h| micro(h.at.y));
        let (x0, x1) = xs.fold((i64::MAX, i64::MIN), |m, v| (m.0.min(v), m.1.max(v)));
        let (y0, y1) = ys.fold((i64::MAX, i64::MIN), |m, v| (m.0.min(v), m.1.max(v)));
        let _ = writeln!(out, "box: {x0} {y0} {x1} {y1}");
    }
    let _ = writeln!(out, "slots: {}", file.slots.len());
    for s in &file.slots {
        let _ = writeln!(
            out,
            "  T{} {} {} -> {} {}",
            s.tool,
            micro(s.from.x),
            micro(s.from.y),
            micro(s.to.x),
            micro(s.to.y)
        );
    }
    let _ = writeln!(out, "routes: {}", file.routes.len());
    for r in &file.routes {
        let _ = writeln!(
            out,
            "  tool={:?} points={} arcs={}",
            r.tool,
            r.points.len(),
            r.arcs.len()
        );
        for a in &r.arcs {
            let _ = writeln!(
                out,
                "    arc to={} cw={} radius={} offset={:?}",
                a.to_index,
                a.clockwise,
                a.radius.map_or(-1, micro),
                a.center_offset.map(|(i, j)| (micro(i), micro(j)))
            );
        }
    }
    let _ = writeln!(out, "diagnostics: {}", file.diagnostics.len());
    for d in &file.diagnostics {
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
    fn the_text_is_the_same_every_time_and_has_whole_numbers_only() {
        let text =
            "M48\n; #@! TF.FileFunction,Plated,1,2,PTH\nMETRIC\nT1C0.8\n%\nT1\nX1.5Y2.25\nM30\n";
        let a = normalize(&crate::parse(text));
        assert_eq!(a, normalize(&crate::parse(text)));
        assert!(a.contains("plating: Some(Plated)"));
        assert!(a.contains("T1 d=800000 holes=1"));
        assert!(a.contains("box: 1500000 2250000 1500000 2250000"));
        // no length is written with a decimal point (the name of an attribute has one)
        assert!(a
            .lines()
            .filter(|l| l.starts_with("box") || l.trim_start().starts_with("T1"))
            .all(|l| !l.contains('.')));
        assert_eq!(micro(f64::NAN), 0);
    }
}
