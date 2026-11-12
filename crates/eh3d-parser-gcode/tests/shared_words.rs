//! The words of a line of G-code, as the server of the suite reads them: a table of lines and what each one gives, the same table that the tests of EH3D-SERVER check against its
//! own tokenizer (`gcode::words`). While the two readers are not one, a change that makes them disagree on one of these lines fails in the repository that made it.

use eh3d_parser_gcode::{parse, Dialect};

/// The lines and the words (letter and value) that both readers give for each.
const CASES: &[(&str, &[(char, f64)])] = &[
    (
        "G1 X10 Y-5.5 F300",
        &[('G', 1.0), ('X', 10.0), ('Y', -5.5), ('F', 300.0)],
    ),
    ("g0x1y2", &[('G', 0.0), ('X', 1.0), ('Y', 2.0)]),
    (
        "G1 X1 (a note) Y2 ; the rest",
        &[('G', 1.0), ('X', 1.0), ('Y', 2.0)],
    ),
    ("M3 S12000", &[('M', 3.0), ('S', 12000.0)]),
    ("G21G90", &[('G', 21.0), ('G', 90.0)]),
    ("G1 X-0.25 Z-1", &[('G', 1.0), ('X', -0.25), ('Z', -1.0)]),
];

#[test]
fn the_lines_of_the_shared_table_give_the_words_the_server_gives() {
    for (line, expected) in CASES {
        let program = parse(line, Dialect::Grbl);
        let block = program
            .blocks
            .first()
            .unwrap_or_else(|| panic!("{line:?}: no block"));
        let got: Vec<(char, f64)> = block.words.iter().map(|w| (w.letter, w.value)).collect();
        assert_eq!(&got, expected, "{line:?}");
    }
}
