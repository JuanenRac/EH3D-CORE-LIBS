//! The one way the programs of the suite write a number and the header of a file.

use eh3d_parser_gcode::emit::{header, number, sha256_hex, Header};
use eh3d_parser_gcode::{interpret, parse, Dialect};

#[test]
fn a_number_is_short_has_no_negative_zero_and_is_never_nan_or_inf() {
    assert_eq!(number(12.0).unwrap(), "12");
    assert_eq!(number(-0.0004).unwrap(), "0");
    assert_eq!(number(1.2346).unwrap(), "1.235");
    // 1.2345 is stored a hair under that value, so it rounds down: the text is what the number really is
    assert_eq!(number(1.2345).unwrap(), "1.234");
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(number(bad).is_err());
    }
}

#[test]
fn what_is_written_is_read_back_by_the_reader_of_the_same_crate() {
    let h = header(&Header {
        generator: "g",
        version: "1",
        source: Some(("in.gbr", sha256_hex(b"x"))),
        tools: "t",
    });
    let program = format!(
        "{h}G21\nG90\nG1 X{} Y{} F100\nM2\n",
        number(1.5).unwrap(),
        number(-2.0).unwrap()
    );
    let p = parse(&program, Dialect::Grbl);
    assert!(p.is_clean(), "{:?}", p.diagnostics);
    let t = interpret(&p);
    assert_eq!(t.segments.len(), 1);
    assert_eq!((t.segments[0].to.x, t.segments[0].to.y), (1.5, -2.0));
}
