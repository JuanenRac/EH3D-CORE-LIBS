//! More of the G-code reader: comments, the checksum of Marlin, long lines, signs and the end of the program.

use eh3d_parser_gcode::{codes, interpret, parse, parse_bytes, Dialect, Severity};

#[test]
fn a_star_inside_a_comment_is_not_a_checksum() {
    for text in ["G1 X1 F100 ; speed*2\n", "G1 X1 F100 ;*\n"] {
        let p = parse(text, Dialect::Marlin);
        assert!(p.is_clean(), "{text:?}: {:?}", p.diagnostics);
        assert_eq!(p.blocks[0].get('X'), Some(1.0));
    }
    // the same star after the words is still a checksum, here a wrong one
    let bad = parse("G1 X1 F100*1 ; comment\n", Dialect::Marlin);
    assert!(bad
        .diagnostics
        .iter()
        .any(|d| d.code == codes::CHECKSUM_MISMATCH));
}

#[test]
fn a_checksum_that_is_not_a_number_is_a_warning_and_the_words_are_still_read() {
    let p = parse("G1 X5 F100*abc\n", Dialect::Marlin);
    assert!(p
        .diagnostics
        .iter()
        .any(|d| d.code == codes::CHECKSUM_INVALID && d.severity == Severity::Warning));
    assert_eq!(p.blocks[0].get('X'), Some(5.0));
}

#[test]
fn comments_of_both_kinds_are_collected_and_joined() {
    let p = parse("G1 X1 (first) Y2 (second) ; third\n", Dialect::Grbl);
    assert_eq!(p.blocks[0].comment.as_deref(), Some("first second third"));
    assert_eq!(
        (p.blocks[0].get('X'), p.blocks[0].get('Y')),
        (Some(1.0), Some(2.0))
    );
    let only = parse("(only a comment)\n; and another\n", Dialect::Grbl);
    assert!(
        only.blocks.is_empty(),
        "a line without words is not a block"
    );
}

#[test]
fn a_comment_in_parentheses_that_is_never_closed_runs_to_the_end_with_a_warning() {
    let p = parse("G1 X1 (never closed Y2\nG1 X3\n", Dialect::Grbl);
    assert!(p
        .diagnostics
        .iter()
        .any(|d| d.code == codes::COMMENT_UNCLOSED && d.line == 1));
    assert_eq!(p.blocks[0].get('Y'), None);
    assert_eq!(
        p.blocks[1].get('X'),
        Some(3.0),
        "the next line is not swallowed"
    );
}

#[test]
fn words_written_without_spaces_in_lower_case_are_the_same_words() {
    let a = parse("G1X1.5Y-2F300\n", Dialect::Grbl);
    let b = parse("g1 x1.5 y-2 f300\n", Dialect::Grbl);
    assert_eq!(a.blocks[0].words, b.blocks[0].words);
    assert_eq!(a.blocks[0].words.len(), 4);
}

#[test]
fn a_line_with_multibyte_characters_is_cut_at_a_character_not_in_the_middle_of_one() {
    let long = format!("G1 X1 ({})\n", "é".repeat(700));
    let p = parse(&long, Dialect::Grbl);
    assert!(p.diagnostics.iter().any(|d| d.code == codes::LINE_TOO_LONG));
    assert_eq!(p.blocks[0].get('X'), Some(1.0));
    let comment = p.blocks[0].comment.clone().unwrap_or_default();
    assert!(comment.chars().all(|c| c == 'é'));
    assert!(comment.chars().count() <= 512);
    let exact = format!("G1 X1 {}\n", " ".repeat(506));
    assert!(
        parse(&exact, Dialect::Grbl)
            .diagnostics
            .iter()
            .all(|d| d.code != codes::LINE_TOO_LONG),
        "512 characters is the longest line that is not cut"
    );
}

#[test]
fn a_block_delete_mark_is_read_as_if_the_switch_were_off() {
    let p = parse("/G1 X4 F100\n", Dialect::Grbl);
    assert_eq!(p.blocks[0].get('X'), Some(4.0));
    assert!(p
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_DELETE_READ && d.severity == Severity::Info));
    assert!(p.is_clean());
}

#[test]
fn numbers_with_signs_dots_and_leading_zeros_are_words() {
    let p = parse("G01 X+007.50 Y-.25 Z5.\n", Dialect::Grbl);
    let b = &p.blocks[0];
    assert_eq!(
        (b.get('G'), b.get('X'), b.get('Y'), b.get('Z')),
        (Some(1.0), Some(7.5), Some(-0.25), Some(5.0))
    );
}

#[test]
fn a_word_with_no_number_is_an_error_named_with_its_line_and_the_rest_of_the_line_is_read() {
    let p = parse("G1 X1\nG1 X Y2\n", Dialect::Grbl);
    let e: Vec<_> = p
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert_eq!(e.len(), 1, "{:?}", p.diagnostics);
    assert_eq!(e[0].line, 2);
    assert_eq!(p.blocks[1].get('Y'), Some(2.0));
}

#[test]
fn the_byte_order_mark_and_non_utf8_comments_do_not_change_the_moves() {
    let program = "G21 G90\nG1 X10 F100\nM2\n";
    let want = interpret(&parse(program, Dialect::Grbl));
    let mut marked = vec![0xEF, 0xBB, 0xBF];
    marked.extend_from_slice(program.as_bytes());
    let got = interpret(&parse_bytes(&marked, Dialect::Grbl).unwrap());
    assert_eq!(got.segments, want.segments);
    let mut latin = b"(caf".to_vec();
    latin.push(0xE9);
    latin.extend_from_slice(b")\n");
    latin.extend_from_slice(program.as_bytes());
    let parsed = parse_bytes(&latin, Dialect::Grbl).unwrap();
    assert!(parsed
        .diagnostics
        .iter()
        .any(|d| d.code == codes::INPUT_NOT_UTF8));
    let moves = |t: &eh3d_parser_gcode::Trace| {
        t.segments
            .iter()
            .map(|s| (s.from, s.to, s.feed))
            .collect::<Vec<_>>()
    };
    assert_eq!(moves(&interpret(&parsed)), moves(&want));
}

#[test]
fn the_same_program_followed_twice_gives_the_same_trace() {
    let text = "G21 G90\nG0 X5 Y5\nG1 Z-1 F100\nG2 X10 Y5 I2.5 J0\nG1 X0\nM30\n";
    let a = interpret(&parse(text, Dialect::Grbl));
    let b = interpret(&parse(text, Dialect::Grbl));
    assert_eq!(a.segments, b.segments);
    assert_eq!(
        (a.cutting_length_mm, a.rapid_length_mm, a.ended),
        (b.cutting_length_mm, b.rapid_length_mm, b.ended)
    );
}

#[test]
fn the_lengths_of_a_trace_add_up_from_its_segments() {
    let t = interpret(&parse(
        "G21 G90\nG0 X30 Y40\nG1 X30 Y0 F100\nG1 X0 Y0\nG0 Z10\n",
        Dialect::Grbl,
    ));
    assert!(
        (t.rapid_length_mm - 60.0).abs() < 1e-9,
        "{}",
        t.rapid_length_mm
    );
    assert!(
        (t.cutting_length_mm - 70.0).abs() < 1e-9,
        "{}",
        t.cutting_length_mm
    );
    assert_eq!(t.segments.len(), 4);
}
