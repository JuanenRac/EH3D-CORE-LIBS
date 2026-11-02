//! Reads every synthetic Excellon file of `tests/golden/excellon` and compares what the reader found
//! with the `.expected` summary next to it. `EH3D_REGENERATE=1 cargo test` rewrites the summaries,
//! so the change shows in the diff.

use eh3d_parser_excellon::{parse_bytes, ExcellonFile, Severity};
use std::fmt::Write as _;
use std::path::PathBuf;

mod common;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/excellon")
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

fn summary(file: &ExcellonFile) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "units: {:?}  zeros: {:?}  ended: {}",
        file.units, file.zeros, file.ended
    );
    for tool in file.tools.values() {
        let _ = writeln!(out, "tool T{} diameter {:.4}", tool.number, tool.diameter);
    }
    let _ = writeln!(
        out,
        "drills: {}  slots: {}  routes: {} ({} points)",
        file.drills.len(),
        file.slots.len(),
        file.routes.len(),
        file.routes.iter().map(|r| r.points.len()).sum::<usize>()
    );
    let mut xs: Vec<f64> = file.drills.iter().map(|h| h.at.x).collect();
    let mut ys: Vec<f64> = file.drills.iter().map(|h| h.at.y).collect();
    for s in &file.slots {
        xs.extend([s.from.x, s.to.x]);
        ys.extend([s.from.y, s.to.y]);
    }
    for r in &file.routes {
        xs.extend(r.points.iter().map(|p| p.x));
        ys.extend(r.points.iter().map(|p| p.y));
    }
    if xs.is_empty() {
        out.push_str(
            "box: none
",
        );
    } else {
        let lo = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let _ = writeln!(
            out,
            "box: x {:.4}..{:.4}  y {:.4}..{:.4}",
            lo(&xs),
            hi(&xs),
            lo(&ys),
            hi(&ys)
        );
    }
    let _ = writeln!(out, "diagnostics: {}", file.diagnostics.len());
    for d in &file.diagnostics {
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
        let file = parse_bytes(&bytes).expect("small file");
        let got = summary(&file);
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
        if name.starts_with("cad_") {
            let file = parse_bytes(&std::fs::read(&path).unwrap()).unwrap();
            let errors: Vec<_> = file
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
        common::no_variant_breaks_the_reader(&[text.as_str()], |s| {
            let _ = eh3d_parser_excellon::parse(s);
        });
    }
}
