//! `G92`, `G28`, `G30`, `G53` and `G54`..`G59` are followed by `interpret_coordinates` and by nothing else.

use eh3d_parser_gcode::{
    codes, interpret_coordinates, interpret_with, parse, parse_bytes, CoordinateOptions,
    CoordinateTrace, Dialect, InterpretOptions, Motion, Point3, Severity,
};
use std::path::PathBuf;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn point(x: f64, y: f64, z: f64) -> Point3 {
    Point3 { x, y, z }
}

fn at(p: Point3, x: f64, y: f64, z: f64) -> bool {
    close(p.x, x) && close(p.y, y) && close(p.z, z)
}

fn with_work_offset(index: usize, origin: Point3) -> CoordinateOptions {
    let mut work_offsets = [Point3::default(); 6];
    work_offsets[index] = origin;
    CoordinateOptions {
        work_offsets,
        ..CoordinateOptions::default()
    }
}

fn walk_with(text: &str, dialect: Dialect, options: &CoordinateOptions) -> CoordinateTrace {
    interpret_coordinates(&parse(text, dialect), &InterpretOptions::default(), options)
}

fn walk(text: &str) -> CoordinateTrace {
    walk_with(text, Dialect::Grbl, &CoordinateOptions::default())
}

fn codes_of(t: &CoordinateTrace) -> Vec<&'static str> {
    t.trace.diagnostics.iter().map(|d| d.code).collect()
}

#[test]
fn g92_shifts_the_positions_that_follow() {
    let t = walk("G1 X10 Y10 F100\nG92 X0 Y0\nG1 X5 F100\n");
    assert!(t.trace.diagnostics.is_empty(), "{:?}", t.trace.diagnostics);
    assert_eq!(t.trace.segments.len(), 2);
    assert!(at(t.trace.segments[1].to, 15.0, 10.0, 0.0));
    assert!(at(t.state.g92_offset, 10.0, 10.0, 0.0));
    assert!(t.state.g92_active);
    assert!(at(t.state.offset, 10.0, 10.0, 0.0));
}

#[test]
fn g92_does_not_add_up_when_it_is_repeated() {
    let t = walk("G1 X10 F100\nG92 X0\nG92 X3\nG1 X4\n");
    assert!(at(t.trace.segments[1].to, 11.0, 0.0, 0.0));
    assert!(close(t.state.g92_offset.x, 7.0));
}

#[test]
fn g92_1_cancels_the_offset() {
    let t = walk("G1 X10 F100\nG92 X0\nG92.1\nG1 X5\n");
    assert!(at(t.trace.segments[1].to, 5.0, 0.0, 0.0));
    assert!(!t.state.g92_active);
    assert!(at(t.state.g92_offset, 0.0, 0.0, 0.0));
    assert!(at(t.state.offset, 0.0, 0.0, 0.0));
}

#[test]
fn g92_2_suspends_and_g92_3_applies_it_again() {
    let t = walk("G1 X10 F100\nG92 X0\nG92.2\nG1 X5\nG92.3\nG1 X5\n");
    let xs: Vec<f64> = t.trace.segments.iter().map(|s| s.to.x).collect();
    assert_eq!(xs.len(), 3);
    assert!(
        close(xs[0], 10.0) && close(xs[1], 5.0) && close(xs[2], 15.0),
        "{xs:?}"
    );
    assert!(t.state.g92_active);
}

#[test]
fn g92_reads_its_values_in_the_units_of_the_program() {
    let t = walk("G20\nG1 X1 F10\nG92 X0\nG21\nG1 X5\n");
    assert!(at(t.trace.segments[1].to, 30.4, 0.0, 0.0));
}

#[test]
fn g92_without_axes_is_told_but_an_extruder_axis_is_not() {
    let t = walk("G92\n");
    assert_eq!(codes_of(&t), vec![codes::G92_AXES_MISSING]);
    assert_eq!(t.trace.diagnostics[0].severity, Severity::Warning);
    let t = walk("G92 E0\n");
    assert!(t.trace.diagnostics.is_empty(), "{:?}", t.trace.diagnostics);
    assert!(!t.state.g92_active);
}

#[test]
fn g92_offset_above_the_limit_is_refused() {
    let options = CoordinateOptions {
        max_offset_mm: 5.0,
        ..CoordinateOptions::default()
    };
    let t = walk_with("G1 X10 F100\nG92 X0\n", Dialect::Grbl, &options);
    assert_eq!(codes_of(&t), vec![codes::OFFSET_OUT_OF_RANGE]);
    assert_eq!(t.trace.diagnostics[0].severity, Severity::Error);
    assert!(close(t.state.g92_offset.x, 0.0));
    assert!(!t.state.g92_active);
}

#[test]
fn g53_moves_in_machine_coordinates_for_its_block_only() {
    let options = with_work_offset(0, point(100.0, 50.0, 0.0));
    let t = walk_with("G0 X0 Y0\nG53 G0 X10\nG0 X1\n", Dialect::Grbl, &options);
    assert!(t.trace.diagnostics.is_empty(), "{:?}", t.trace.diagnostics);
    assert!(at(t.trace.segments[0].to, 100.0, 50.0, 0.0));
    assert!(at(t.trace.segments[1].to, 10.0, 50.0, 0.0));
    assert!(at(t.trace.segments[2].to, 101.0, 50.0, 0.0));
}

#[test]
fn g53_with_an_arc_is_an_error_and_moves_nothing() {
    let t = walk("G2 X1 Y1 I1 J0 F100\nG53 X5\n");
    assert_eq!(t.trace.segments.len(), 1);
    assert_eq!(codes_of(&t), vec![codes::G53_MOTION_UNSUPPORTED]);
    assert_eq!(t.trace.diagnostics[0].severity, Severity::Error);
}

#[test]
fn work_systems_use_their_configured_origin() {
    let options = with_work_offset(1, point(10.0, 20.0, 0.0));
    let t = walk_with(
        "G55\nG0 X1 Y1\nG54\nG0 X1 Y1\nG55\n",
        Dialect::Grbl,
        &options,
    );
    assert!(at(t.trace.segments[0].to, 11.0, 21.0, 0.0));
    assert!(at(t.trace.segments[1].to, 1.0, 1.0, 0.0));
    assert_eq!(t.state.work_system, 55);
    assert!(at(t.state.work_offset, 10.0, 20.0, 0.0));
    assert!(at(t.state.offset, 10.0, 20.0, 0.0));
}

#[test]
fn an_absolute_arc_centre_follows_the_work_origin() {
    let options = with_work_offset(0, point(10.0, 0.0, 0.0));
    let t = walk_with(
        "G90.1\nG0 X0 Y0\nG2 X10 Y0 I5 J0 F100\n",
        Dialect::Grbl,
        &options,
    );
    assert!(t.trace.diagnostics.is_empty(), "{:?}", t.trace.diagnostics);
    let Motion::ArcCw { center } = t.trace.segments[1].motion else {
        panic!("not an arc: {:?}", t.trace.segments[1]);
    };
    assert!(close(center.0, 15.0) && close(center.1, 0.0), "{center:?}");
}

#[test]
fn g28_goes_through_the_point_and_then_to_the_reference_position() {
    let options = CoordinateOptions {
        reference_g28: Some(point(0.0, 0.0, 50.0)),
        ..CoordinateOptions::default()
    };
    let t = walk_with(
        "G1 X10 Y10 Z-1 F100\nG28 Z5\nG1 X1\n",
        Dialect::Grbl,
        &options,
    );
    assert!(t.trace.diagnostics.is_empty(), "{:?}", t.trace.diagnostics);
    let s = &t.trace.segments;
    assert_eq!(s.len(), 4);
    assert_eq!(s[1].motion, Motion::Rapid);
    assert!(at(s[1].to, 10.0, 10.0, 5.0));
    assert_eq!(s[2].motion, Motion::Rapid);
    assert!(at(s[2].to, 10.0, 10.0, 50.0));
    assert_eq!(s[3].motion, Motion::Linear);
    assert!(at(s[3].to, 1.0, 10.0, 50.0));
}

#[test]
fn g28_without_axes_sends_every_axis_home() {
    let options = CoordinateOptions {
        reference_g28: Some(point(0.0, 0.0, 50.0)),
        ..CoordinateOptions::default()
    };
    let t = walk_with("G1 X10 Y10 F100\nG28\n", Dialect::Grbl, &options);
    assert_eq!(t.trace.segments.len(), 2);
    assert_eq!(t.trace.segments[1].motion, Motion::Rapid);
    assert!(at(t.trace.segments[1].to, 0.0, 0.0, 50.0));
}

#[test]
fn g30_has_its_own_position() {
    let options = CoordinateOptions {
        reference_g28: Some(point(9.0, 9.0, 9.0)),
        reference_g30: Some(point(1.0, 2.0, 3.0)),
        ..CoordinateOptions::default()
    };
    let t = walk_with("G30\n", Dialect::Grbl, &options);
    assert_eq!(t.trace.segments.len(), 1);
    assert!(at(t.trace.segments[0].to, 1.0, 2.0, 3.0));
}

#[test]
fn g28_without_a_configured_position_moves_nothing_and_says_so() {
    let t = walk("G1 X10 F100\nG28 Z5\nG30\n");
    assert_eq!(t.trace.segments.len(), 1);
    assert_eq!(
        codes_of(&t),
        vec![
            codes::REFERENCE_POSITION_UNSET,
            codes::REFERENCE_POSITION_UNSET
        ]
    );
    assert_eq!(t.trace.diagnostics[0].severity, Severity::Warning);
}

#[test]
fn the_point_of_g28_is_in_the_work_coordinates_with_the_g92_offset() {
    let options = CoordinateOptions {
        reference_g28: Some(point(7.0, 0.0, 0.0)),
        ..CoordinateOptions::default()
    };
    let t = walk_with("G92 X5\nG28 X3\n", Dialect::Grbl, &options);
    let s = &t.trace.segments;
    assert_eq!(s.len(), 2);
    assert!(at(s[0].to, -2.0, 0.0, 0.0));
    assert!(at(s[1].to, 7.0, 0.0, 0.0));
}

#[test]
fn marlin_g28_only_sends_the_axes_named_home() {
    let options = CoordinateOptions {
        reference_g28: Some(point(0.0, 0.0, 0.0)),
        ..CoordinateOptions::default()
    };
    let t = walk_with("G1 X10 Y10 F100\nG28 X0\n", Dialect::Marlin, &options);
    assert_eq!(t.trace.segments.len(), 2);
    assert!(at(t.trace.segments[1].to, 0.0, 10.0, 0.0));
}

#[test]
fn a_program_that_does_not_use_them_gives_the_same_trace() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/gcode");
    let mut compared = 0;
    for entry in std::fs::read_dir(dir).expect("the golden folder") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|x| x == "expected") {
            continue;
        }
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        let bytes = std::fs::read(&path).expect("the file");
        let dialect = if name.starts_with("marlin") {
            Dialect::Marlin
        } else {
            Dialect::Grbl
        };
        let program = parse_bytes(&bytes, dialect).expect("a readable file");
        let uses_them = program.blocks.iter().any(|b| {
            b.words.iter().any(|w| {
                w.letter == 'G'
                    && matches!((w.value * 10.0).round() as i64, 280 | 300 | 530 | 920..=923)
            })
        });
        if uses_them {
            continue;
        }
        let options = InterpretOptions::default();
        let plain = interpret_with(&program, &options);
        let followed = interpret_coordinates(&program, &options, &CoordinateOptions::default());
        assert_eq!(plain, followed.trace, "{name}");
        compared += 1;
    }
    assert!(compared >= 4, "only {compared} files were compared");
}

#[test]
fn the_default_options_invent_nothing() {
    let plain = interpret_with(
        &parse("G54\nG1 X5 F10\nG55\nG1 Y5\n", Dialect::Grbl),
        &InterpretOptions::default(),
    );
    let followed = walk("G54\nG1 X5 F10\nG55\nG1 Y5\n");
    assert_eq!(plain, followed.trace);
    assert_eq!(followed.state.work_system, 55);
}
