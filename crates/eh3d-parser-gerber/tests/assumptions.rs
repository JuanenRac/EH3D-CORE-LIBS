//! What a Gerber file says twice, differently or after its end, and what is left out of a block, is told.

use eh3d_parser_gerber::{codes, parse, GerberFile, Operation, Severity};

const HEAD: &str = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX0Y0D02*\n";

fn told(file: &GerberFile, code: &str) -> Vec<(usize, Severity)> {
    file.diagnostics
        .iter()
        .filter(|d| d.code == code)
        .map(|d| (d.line, d.severity))
        .collect()
}

fn with(lines: &str) -> GerberFile {
    parse(&format!("{HEAD}{lines}\nM02*\n"))
}

#[test]
fn a_clean_layer_says_nothing_new() {
    let file = with("%MOMM*%\nG71*\n%FSLAX24Y24*%\nX10000Y0D01*");
    for code in [
        codes::UNITS_CONFLICT,
        codes::FORMAT_REDEFINED,
        codes::TEXT_AFTER_END,
        codes::CHARACTER_IGNORED,
        codes::WORD_REPEATED,
    ] {
        assert!(told(&file, code).is_empty(), "{code}");
    }
    let comment = parse(&format!("{HEAD}M02*\nG04 the end*\nM02*\n"));
    assert!(told(&comment, codes::TEXT_AFTER_END).is_empty());
}

#[test]
fn units_and_format_declared_again_and_differently_are_errors() {
    for again in ["%MOIN*%", "G70*"] {
        assert_eq!(
            told(&with(again), codes::UNITS_CONFLICT),
            [(6, Severity::Error)],
            "{again}"
        );
    }
    assert_eq!(
        told(&with("%FSLAX36Y36*%"), codes::FORMAT_REDEFINED),
        [(6, Severity::Error)]
    );
}

#[test]
fn commands_after_the_end_are_told_once_and_still_read() {
    let file = parse(&format!("{HEAD}M02*\nX10000Y0D01*\nX20000Y0D01*\n"));
    assert_eq!(told(&file, codes::TEXT_AFTER_END), [(7, Severity::Warning)]);
    assert_eq!(
        file.commands
            .iter()
            .filter(|c| matches!(c.op, Operation::Line { .. }))
            .count(),
        2
    );
}

#[test]
fn what_is_left_out_of_a_block_or_written_twice_is_an_error() {
    for (block, code) in [
        ("10000Y0D01*", codes::CHARACTER_IGNORED),
        ("X1X10000Y0D01*", codes::WORD_REPEATED),
        ("X10000Y0D01D02*", codes::WORD_REPEATED),
    ] {
        let file = with(block);
        assert_eq!(told(&file, code), [(6, Severity::Error)], "{block}");
        assert!(!file.is_clean());
    }
}
