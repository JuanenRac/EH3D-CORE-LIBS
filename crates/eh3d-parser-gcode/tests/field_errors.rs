//! Regression tests of faults that other CAM programs had in the field and that the G-code side of this suite can have:
//! a decimal comma from the language of the computer in the numbers written, a cutting depth of zero, settings kept from
//! one job to the next, a program that does not end, and a structure of several depths that refers to itself.

use eh3d_parser_gcode::emit::number;
use eh3d_parser_gcode::{interpret, parse, parse_with_limits, Dialect, ModalState, ParseLimits};

/// A program that writes its numbers with the language of the computer wrote `G1 X1,5`, which no controller reads.
/// The numbers of this suite are written by one function that never looks at the language and never writes a comma.
#[test]
fn a_number_is_written_with_a_point_whatever_the_language() {
    for (v, text) in [
        (1.5, "1.5"),
        (-0.25, "-0.25"),
        (1234.5678, "1234.568"),
        (0.0004, "0"),
        (100.0, "100"),
    ] {
        assert_eq!(number(v).unwrap(), text);
    }
    for v in [0.0, 1.0e-9, -3.14158, 12345.678, 1.0e7] {
        let text = number(v).unwrap();
        assert!(!text.contains(','), "{text}");
        assert!(!text.contains('e') && !text.contains('E'), "{text}");
    }
    // a comma in a program is not a number: it is said, not guessed
    let program = parse("G21\nG1 X1,5 Y2\n", Dialect::Grbl);
    assert!(!program.is_clean() || program.blocks.iter().all(|b| b.get('X') != Some(1.5)));
}

/// A cutting depth of zero ended in an exception instead of a clean answer. Writing and reading a depth of zero, and
/// a motion that stays at zero, are ordinary.
#[test]
fn a_depth_of_zero_is_written_read_and_interpreted_cleanly() {
    assert_eq!(number(0.0).unwrap(), "0");
    assert_eq!(number(-0.0).unwrap(), "0", "no minus zero");
    assert!(number(f64::NAN).is_err() && number(f64::INFINITY).is_err());
    let program = parse(
        "G21\nG90\nG0 Z0\nG1 Z0 F100\nG1 X10 Z0\nG1 Z-0\nM30\n",
        Dialect::Grbl,
    );
    assert!(program.is_clean(), "{:?}", program.diagnostics);
    let trace = interpret(&program);
    assert!(trace
        .segments
        .iter()
        .all(|s| s.from.z == 0.0 && s.to.z == 0.0));
    assert_eq!(trace.cutting_length_mm, 10.0);
}

/// Settings of the previous job leaked into the next. A modal state is a value: a new one is empty, and reading a
/// program does not touch another.
#[test]
fn the_modal_state_is_a_value_and_nothing_is_kept_between_programs() {
    let a = parse("G20\nG91\nG55\nM3 S1000\nM8\nG1 X1 F50\n", Dialect::Grbl);
    let mut state = ModalState::default();
    for block in &a.blocks {
        state.apply_block(block);
    }
    assert_eq!(state.units, Some(200));
    assert_eq!(state.distance, Some(910));
    let fresh = ModalState::default();
    assert_eq!(fresh, ModalState::default());
    assert!(fresh.units.is_none() && fresh.feed.is_none() && fresh.spindle.is_none());
    let b = parse("G1 X1\n", Dialect::Grbl);
    assert_eq!(parse("G1 X1\n", Dialect::Grbl), b);
    // after the messy program the plain one gives the same trace as alone
    let alone = interpret(&b);
    let _ = interpret(&a);
    assert_eq!(interpret(&b), alone);
}

/// A program that does not end: the work is bounded by the limits and the reason is named.
#[test]
fn a_program_of_millions_of_blocks_ends_with_a_named_reason() {
    let mut text = String::from("G21\nG90\n");
    for i in 0..100_000 {
        text.push_str(&format!("G1 X{} Y{} F100\n", i % 100, (i / 100) % 100));
    }
    assert_eq!(parse(&text, Dialect::Grbl).blocks.len(), 100_002);
    let cut = parse_with_limits(
        &text,
        Dialect::Grbl,
        &ParseLimits {
            max_blocks: 50,
            ..ParseLimits::default()
        },
    );
    assert_eq!(cut.blocks.len(), 50);
    assert!(cut
        .diagnostics
        .iter()
        .any(|d| d.code == eh3d_parser_gcode::codes::PROGRAM_TOO_LONG));
}

/// A job saved after several depths failed with a circular reference. A trace of many passes at many depths is a plain
/// value: it can be cloned, compared and printed.
#[test]
fn a_trace_of_many_depths_is_a_plain_value() {
    let mut text = String::from("G21\nG90\nG0 Z5\n");
    for pass in 1..=12 {
        text.push_str(&format!(
            "G0 X0 Y0\nG1 Z-{} F60\nG1 X20 F200\nG1 Y20\nG1 X0\nG1 Y0\nG0 Z5\n",
            f64::from(pass) * 0.1
        ));
    }
    text.push_str("M30\n");
    let program = parse(&text, Dialect::Grbl);
    assert!(program.is_clean(), "{:?}", program.diagnostics);
    let trace = interpret(&program);
    let copy = trace.clone();
    assert_eq!(copy, trace);
    assert!(!format!("{trace:?}").is_empty());
    assert_eq!(
        trace
            .segments
            .iter()
            .filter(|s| s.to.z < 0.0 && s.from.z < 0.0)
            .count(),
        12 * 4
    );
    assert!((trace.min.z + 1.2).abs() < 1e-9);
}
