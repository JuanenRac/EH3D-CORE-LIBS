//! Reads every synthetic G-code file of `tests/golden/gcode` and compares what the reader found
//! with the `.expected` summary next to it. `EH3D_REGENERATE=1 cargo test` rewrites the summaries,
//! so the change shows in the diff.

use eh3d_parser_gcode::{interpret, parse_bytes, Dialect, Severity, Trace};
use std::fmt::Write as _;
use std::path::PathBuf;

mod common;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/gcode")
}

fn files() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(golden_dir())
        .expect("the golden folder")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x != "expected"))
        .collect();
    found.sort();
    found
}

fn dialect_of(name: &str) -> Dialect {
    if name.starts_with("marlin") {
        Dialect::Marlin
    } else {
        // LinuxCNC programs are read with the GRBL dialect: both use `;` and `( )` comments
        Dialect::Grbl
    }
}

fn summary(blocks: usize, trace: &Trace) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "blocks: {blocks}  segments: {}  ended: {}",
        trace.segments.len(),
        trace.ended
    );
    let _ = writeln!(
        out,
        "box: x {:.4}..{:.4}  y {:.4}..{:.4}  z {:.4}..{:.4}",
        trace.min.x, trace.max.x, trace.min.y, trace.max.y, trace.min.z, trace.max.z
    );
    let _ = writeln!(
        out,
        "cutting: {:.3} mm  rapid: {:.3} mm",
        trace.cutting_length_mm, trace.rapid_length_mm
    );
    let _ = writeln!(out, "diagnostics: {}", trace.diagnostics.len());
    for d in &trace.diagnostics {
        let _ = writeln!(out, "  line {} {:?} {}", d.line, d.severity, d.code);
    }
    out
}

#[test]
fn every_golden_file_gives_its_summary() {
    let regenerate = std::env::var_os("EH3D_REGENERATE").is_some();
    let found = files();
    assert!(found.len() >= 6, "the corpus is missing");
    for path in found {
        let bytes = std::fs::read(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let program = parse_bytes(&bytes, dialect_of(&name)).expect("small file");
        let got = summary(program.blocks.len(), &interpret(&program));
        let expected_path = PathBuf::from(format!("{}.expected", path.display()));
        if regenerate {
            std::fs::write(&expected_path, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&expected_path)
            .unwrap_or_else(|_| panic!("{} has no .expected summary", path.display()))
            .replace("\r\n", "\n");
        assert_eq!(got, want, "{}", path.display());
    }
}

#[test]
fn the_clean_files_have_no_error() {
    for path in files() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if name.starts_with("grbl_") || name.starts_with("inches_") {
            let program = parse_bytes(&std::fs::read(&path).unwrap(), dialect_of(&name)).unwrap();
            let trace = interpret(&program);
            let errors: Vec<_> = trace
                .diagnostics
                .iter()
                .filter(|d| d.severity >= Severity::Error)
                .collect();
            assert!(errors.is_empty(), "{name}: {errors:?}");
        }
    }
}

#[test]
fn no_variant_of_a_golden_file_breaks_the_reader() {
    for path in files() {
        let text = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
        let dialect = dialect_of(&path.file_name().unwrap().to_string_lossy());
        common::no_variant_breaks_the_reader(&[text.as_str()], move |s| {
            let _ = interpret(&eh3d_parser_gcode::parse(s, dialect));
        });
    }
}
