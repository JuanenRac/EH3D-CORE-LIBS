//! Reads every synthetic Gerber file of `tests/golden/gerber` and compares what the reader found
//! with the `.expected` summary next to it. `EH3D_REGENERATE=1 cargo test` rewrites the summaries,
//! so the change shows in the diff.

use eh3d_parser_gerber::{parse_bytes, GerberFile, Operation, Point, Severity};
use std::fmt::Write as _;
use std::path::PathBuf;

mod common;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/gerber")
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

fn summary(file: &GerberFile) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "units: {:?}", file.units);
    match file.format {
        Some(f) => {
            let _ = writeln!(
                out,
                "format: {}.{} {:?}",
                f.integer_digits, f.decimal_digits, f.zero_omission
            );
        }
        None => out.push_str("format: none\n"),
    }
    let _ = writeln!(
        out,
        "apertures: {}  macros: {}  attributes: {}  ended: {}",
        file.apertures.len(),
        file.macros.len(),
        file.attributes.len(),
        file.ended
    );
    let (mut flashes, mut lines, mut arcs, mut regions, mut moves) = (0, 0, 0, 0, 0);
    let mut points: Vec<Point> = Vec::new();
    for command in &file.commands {
        match &command.op {
            Operation::Flash { at } => {
                flashes += 1;
                points.push(*at);
            }
            Operation::Line { from, to } => {
                lines += 1;
                points.extend([*from, *to]);
            }
            Operation::Arc { from, to, .. } => {
                arcs += 1;
                points.extend([*from, *to]);
            }
            Operation::RegionStart => regions += 1,
            Operation::Move { .. } => moves += 1,
            _ => {}
        }
    }
    let _ = writeln!(
        out,
        "flashes: {flashes}  lines: {lines}  arcs: {arcs}  regions: {regions}  moves: {moves}"
    );
    if points.is_empty() {
        out.push_str("box: none\n");
    } else {
        let min = |f: fn(&Point) -> f64| points.iter().map(f).fold(f64::INFINITY, f64::min);
        let max = |f: fn(&Point) -> f64| points.iter().map(f).fold(f64::NEG_INFINITY, f64::max);
        let _ = writeln!(
            out,
            "box: x {:.4}..{:.4}  y {:.4}..{:.4}",
            min(|p| p.x),
            max(|p| p.x),
            min(|p| p.y),
            max(|p| p.y)
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
            let errors: Vec<_> = file.diagnostics_from(Severity::Error).collect();
            assert!(errors.is_empty(), "{name}: {errors:?}");
        }
    }
}

#[test]
fn no_variant_of_a_golden_file_breaks_the_reader() {
    for path in files() {
        let text = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
        common::no_variant_breaks_the_reader(&[text.as_str()], |s| {
            let _ = eh3d_parser_gerber::parse(s);
        });
    }
}
