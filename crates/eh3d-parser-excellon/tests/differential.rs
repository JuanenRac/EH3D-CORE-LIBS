//! The same Excellon files read by this crate and by the reader of the Bungard formats of EH3D-COMMON (`ncd`), compared hole by hole. The two readers were written for different
//! files (the Bungard one for the files of the Bungard machines, this one for the files of any CAD), so a difference is not always a fault: this test checks what they must agree
//! on (how many holes each tool drills and in which order the tools come) and lists, without failing, where the positions differ, so the differences are known before any reader is
//! changed.

use eh3d_bungard_formats::ncd;
use eh3d_parser_excellon::{parse_bytes, Severity};
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/excellon")
}

fn files() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .expect("the golden folder")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "drl"))
        .collect();
    found.sort();
    found
}

/// The files that are plain drilling (holes only), which both readers must read the same way; the ones with slots, routes or no header are the other reader's business.
fn plain(file: &eh3d_parser_excellon::ExcellonFile) -> bool {
    file.slots.is_empty()
        && file.routes.is_empty()
        && !file
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
}

#[test]
fn both_readers_find_the_same_holes_for_each_tool_in_the_plain_drill_files() {
    let mut compared = 0;
    for path in files() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let bytes = std::fs::read(&path).unwrap();
        let ours = parse_bytes(&bytes).expect("a small file");
        if !plain(&ours) {
            eprintln!("{name}: not a plain drill file, left to its own reader");
            continue;
        }
        let text: String = bytes.iter().map(|&b| b as char).collect();
        let Ok(theirs) = ncd::parse(&text) else {
            eprintln!("{name}: the Bungard reader refuses it");
            continue;
        };
        compared += 1;
        assert_eq!(
            ours.drills.len(),
            theirs.holes.len(),
            "{name}: the number of holes"
        );
        let ours_tools: Vec<u32> = ours.drills.iter().map(|h| h.tool).collect();
        let theirs_tools: Vec<u32> = theirs.holes.iter().map(|h| u32::from(h.tool)).collect();
        assert_eq!(ours_tools, theirs_tools, "{name}: the tool of each hole");
        // where the holes are: told, not asserted (the units and the zeros are read differently, see docs/UNIFICATION.md)
        let units = ours.effective_units();
        let worst = ours
            .drills
            .iter()
            .zip(&theirs.holes)
            .map(|(a, b)| {
                (units.to_mm(a.at.x) - b.x_mm())
                    .abs()
                    .max((units.to_mm(a.at.y) - b.y_mm()).abs())
            })
            .fold(0.0f64, f64::max);
        eprintln!(
            "{name}: {} holes, the positions differ by up to {worst:.4} mm",
            ours.drills.len()
        );
    }
    assert!(compared > 0, "no file was compared");
}
