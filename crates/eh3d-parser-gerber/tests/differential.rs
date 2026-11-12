//! The outline of a board read by this crate and by the Gerber reader of the Bungard formats of EH3D-COMMON (`gerber::outline`, which turns it into a routing path). The two were
//! written for different uses, so the comparison is made on what both must agree on: the corners that the outline reaches. The ends of the lines and arcs read here must lie inside
//! the box of the path of the other reader (an arc may bulge past its ends, so the box of the other may be bigger, never smaller), and how far apart the boxes are is told.

use eh3d_bungard_formats::gerber;
use eh3d_parser_gerber::{parse_bytes, Operation};
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/gerber")
}

#[test]
fn the_ends_of_the_outline_lie_inside_the_path_the_bungard_reader_makes() {
    let mut compared = 0;
    for entry in std::fs::read_dir(golden_dir()).expect("the golden folder") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !gerber::OUTLINE_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let text: String = bytes.iter().map(|&b| b as char).collect();
        if !gerber::looks_like_outline(&name, &text) {
            eprintln!("{name}: not told as an outline by the Bungard reader");
            continue;
        }
        let Ok(theirs) = gerber::outline(&text, 64) else {
            eprintln!("{name}: the Bungard reader refuses it");
            continue;
        };
        let Some((x0, y0, x1, y1)) = theirs.bounds_mm() else {
            continue;
        };
        let ours = parse_bytes(&bytes).expect("a small file");
        let units = ours.units.unwrap_or(eh3d_parser_gerber::Units::Millimetres);
        let mut points: Vec<(f64, f64)> = Vec::new();
        for c in &ours.commands {
            match &c.op {
                Operation::Move { to } => points.push((to.x, to.y)),
                Operation::Line { from, to } | Operation::Arc { from, to, .. } => {
                    points.push((from.x, from.y));
                    points.push((to.x, to.y));
                }
                _ => {}
            }
        }
        assert!(!points.is_empty(), "{name}: no ends read");
        compared += 1;
        let tolerance = 0.05;
        for (x, y) in points {
            let (x, y) = (units.to_mm(x), units.to_mm(y));
            assert!(
                x >= x0 - tolerance && x <= x1 + tolerance && y >= y0 - tolerance && y <= y1 + tolerance,
                "{name}: the end ({x:.3}, {y:.3}) is outside the box ({x0:.3}, {y0:.3}) to ({x1:.3}, {y1:.3}) of the other reader"
            );
        }
    }
    assert!(compared > 0, "no outline was compared");
}
