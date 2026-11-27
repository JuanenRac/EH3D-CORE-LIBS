//! A program that a controller would run differently from what was read must not come out clean: spaces inside a number, commands of the controller, characters it obeys at
//! once, text after the end, a cut line, words the reader does not understand.

use eh3d_parser_gcode::{
    interpret, parse, Diagnostic, Dialect, Motion, Severity, Trace, MAX_LINE_CHARS,
};

fn run(text: &str) -> Trace {
    interpret(&parse(text, Dialect::Grbl))
}

fn errors(found: &[Diagnostic]) -> Vec<(usize, &'static str)> {
    found
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.line, d.code))
        .collect()
}

#[test]
fn spaces_and_comments_inside_a_number_count_for_nothing_as_for_grbl() {
    let p = parse("G1 X1 000 F6 00 S1(a note)000\n", Dialect::Grbl);
    let b = &p.blocks[0];
    assert_eq!(
        (b.get('X'), b.get('F'), b.get('S')),
        (Some(1000.0), Some(600.0), Some(1000.0))
    );
    assert!(p.is_clean(), "{:?}", p.diagnostics);
    assert_eq!(b.comment.as_deref(), Some("a note"));
    let p = parse("G 1 X 10\n", Dialect::Grbl);
    assert_eq!(p.blocks[0].get('X'), Some(10.0));
}

#[test]
fn a_number_split_by_a_space_is_an_error_where_it_cannot_be_joined() {
    for dialect in [Dialect::Marlin, Dialect::PlanetCnc] {
        let p = parse("G1 X1 000 F600\n", dialect);
        assert!(!p.is_clean(), "{dialect:?}: {:?}", p.diagnostics);
    }
}

#[test]
fn what_is_left_out_of_a_line_is_an_error() {
    for line in [
        "G1 X1 #5=3",
        "G1 X1 [2]",
        "G1 X1 %",
        "G1 X1 \u{18}",
        "G1 X1 !",
    ] {
        let p = parse(&format!("{line}\n"), Dialect::Grbl);
        assert!(!p.is_clean(), "{line:?}: {:?}", p.diagnostics);
    }
}

#[test]
fn a_command_of_the_controller_that_is_not_a_question_is_an_error() {
    for line in [
        "$100=1",
        "$32=0",
        "$H",
        "$X",
        "$J=G91 X10 F100",
        "$RST=*",
        "(x) $ 130 = 9999",
        "$N0=G0 X99",
    ] {
        let p = parse(&format!("{line}\n"), Dialect::Grbl);
        assert!(!p.is_clean(), "{line:?}: {:?}", p.diagnostics);
        assert!(p.blocks.is_empty(), "{line:?}");
    }
    for line in ["$", "$$", "$#", "$G", "$I", "$N"] {
        let p = parse(&format!("{line}\n"), Dialect::Grbl);
        assert!(p.is_clean(), "{line:?}: {:?}", p.diagnostics);
    }
}

#[test]
fn a_character_grbl_obeys_at_once_inside_a_comment_is_told() {
    for line in ["G0 X1 (go on~)", "G0 X1 ; ready?", "G0 X1 (stop!)"] {
        let p = parse(&format!("{line}\n"), Dialect::Grbl);
        assert!(
            p.diagnostics
                .iter()
                .any(|d| d.severity == Severity::Warning),
            "{line:?}: {:?}",
            p.diagnostics
        );
    }
}

#[test]
fn a_carriage_return_inside_a_line_ends_it_as_for_the_controller() {
    // the move after the return is not part of the comment: the controller runs it
    let t = run("G0 X1 ; note\rG0 X999\n");
    assert!(
        t.segments.iter().any(|s| s.to.x == 999.0),
        "{:?}",
        t.segments
    );
}

#[test]
fn a_cut_line_with_code_past_the_limit_is_an_error() {
    let long = format!("G1 X1 F10{}X999\n", " ".repeat(MAX_LINE_CHARS));
    assert!(!parse(&long, Dialect::Grbl).is_clean());
    // only a comment past the limit: nothing of the program was lost
    let comment = format!("G1 X1 F10 ;{}\n", "x".repeat(MAX_LINE_CHARS));
    assert!(parse(&comment, Dialect::Grbl).is_clean());
}

#[test]
fn text_after_the_end_of_the_program_is_an_error() {
    let t = run("G0 X1\nM2\nM3 S1000\nG0 X999\n");
    assert!(errors(&t.diagnostics).iter().any(|&(line, _)| line == 3));
}

#[test]
fn feed_modes_that_change_what_f_means_are_errors() {
    for code in ["G93", "G95"] {
        let t = run(&format!("{code}\nG1 X1 F10\n"));
        assert_eq!(
            errors(&t.diagnostics).len(),
            1,
            "{code}: {:?}",
            t.diagnostics
        );
    }
    assert!(errors(&run("G94\nG1 X1 F10\n").diagnostics).is_empty());
}

#[test]
fn an_unknown_code_with_axis_words_is_an_error_and_invents_no_move() {
    for line in ["G33 Z-5", "G5 X9 Y9", "G1.5 X9", "G73 X9 R1"] {
        let t = run(&format!("G1 X1 F10\n{line}\n"));
        assert_eq!(t.segments.len(), 1, "{line}: {:?}", t.segments);
        assert!(
            !errors(&t.diagnostics).is_empty(),
            "{line}: {:?}",
            t.diagnostics
        );
    }
    assert!(!errors(&run("M3.5\n").diagnostics).is_empty());
}

#[test]
fn after_g80_axis_words_are_not_a_cut() {
    let t = run("G1 X1 F10\nG80\nX9\n");
    assert_eq!(t.segments.len(), 1, "{:?}", t.segments);
    assert_eq!(errors(&t.diagnostics).len(), 1, "{:?}", t.diagnostics);
    let t = run("G1 X1 F10\nG80\nG0 X0\n");
    assert!(errors(&t.diagnostics).is_empty(), "{:?}", t.diagnostics);
    assert_eq!(t.segments.last().map(|s| s.motion), Some(Motion::Rapid));
}

#[test]
fn a_tool_length_offset_is_not_taken_for_a_move_and_is_told() {
    let t = run("G1 X1 F10\nG43.1 Z-5\n");
    assert_eq!(t.segments.len(), 1, "{:?}", t.segments);
    for line in ["G43.1 Z-5", "G43 H1 Z5", "G44 H1"] {
        let t = run(&format!("G0 X1\n{line}\n"));
        assert!(
            t.diagnostics
                .iter()
                .any(|d| d.severity >= Severity::Warning),
            "{line}: {:?}",
            t.diagnostics
        );
    }
}

#[test]
fn a_word_written_twice_and_two_codes_of_one_group_are_errors() {
    for line in [
        "G1 X1 X100 F10",
        "G0 G1 X1 F10",
        "G1 X1 F10 F9000",
        "M3 M5",
        "M3 S1 S1000",
        "G20 G21",
        "G90 G91 X1",
    ] {
        let t = run(&format!("{line}\n"));
        assert!(
            !errors(&t.diagnostics).is_empty(),
            "{line}: {:?}",
            t.diagnostics
        );
    }
    let t = run("G21 G90 G17\nG53 G0 X0\nM3 M8 S10\n");
    assert!(errors(&t.diagnostics).is_empty(), "{:?}", t.diagnostics);
}

#[test]
fn a_word_grbl_does_not_have_is_an_error() {
    // 1e5 is not a number for the controller: it reads X1 and then a word E it refuses
    let t = run("G1 X1e5 F10\n");
    assert!(!errors(&t.diagnostics).is_empty(), "{:?}", t.diagnostics);
    // the extruder of Marlin is a word of its own
    let t = interpret(&parse("G1 X1 E2 F10\n", Dialect::Marlin));
    assert!(errors(&t.diagnostics).is_empty(), "{:?}", t.diagnostics);
}

#[test]
fn a_checksum_that_is_not_a_number_is_an_error() {
    assert!(!parse("G1 X5*abc\n", Dialect::Marlin).is_clean());
}
