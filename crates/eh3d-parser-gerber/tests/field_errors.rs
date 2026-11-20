//! Regression tests of faults that other programs of the same kind had in the field and that a reader of Gerber files
//! can have too: a decimal comma from the language of the computer, state kept from one file to the next, a reading that
//! does not end, a structure that refers to itself, a number that loses precision far from the origin, a pen of no
//! width that is not drawn, and a size of zero that ends in an exception instead of a clean error. Each test says the
//! fault it covers.

use eh3d_parser_gerber::image::{resolve, resolve_with, ResolveLimits, Shape};
use eh3d_parser_gerber::snapshot::{micro, normalize};
use eh3d_parser_gerber::{
    codes, decode_coordinate, parse, parse_with_limits, CoordinateFormat, ParseLimits, ZeroOmission,
};

const HEAD: &str = "%FSLAX24Y24*%\n%MOMM*%\n";

/// A computer set to a language that writes the decimal point as a comma wrote files with commas (and a program of
/// the same kind wrote its G-code the same way). The reader must give the same file with a point or a comma, never
/// depend on the language of the machine, and say that it read a comma.
#[test]
fn a_decimal_comma_gives_the_same_file_as_a_decimal_point() {
    let point = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.25*%\n%ADD11R,1.5X0.8*%\nD10*\nX10000Y5000D02*\nX1.5Y2.25D01*\nD11*\nX3.0Y4.0D03*\nM02*\n";
    let comma = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0,25*%\n%ADD11R,1,5X0,8*%\nD10*\nX10000Y5000D02*\nX1,5Y2,25D01*\nD11*\nX3,0Y4,0D03*\nM02*\n";
    // the reading is made of whole numbers and `str::parse`, which do not look at the language of the process
    let (a, b) = (parse(point), parse(comma));
    assert_eq!(a.apertures, b.apertures);
    let ops = |f: &eh3d_parser_gerber::GerberFile| {
        f.commands.iter().map(|c| c.op.clone()).collect::<Vec<_>>()
    };
    assert_eq!(ops(&a), ops(&b));
    assert!(a.is_clean() && b.is_clean());
    assert!(!a.diagnostics.iter().any(|d| d.code == codes::DECIMAL_COMMA));
    assert!(b.diagnostics.iter().any(|d| d.code == codes::DECIMAL_COMMA));
    // a comma and a point in the same number is not a number
    let both = parse("%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0,2.5*%\nM02*\n");
    assert!(
        !both.apertures.contains_key(&10)
            || both
                .diagnostics
                .iter()
                .any(|d| d.code == codes::APERTURE_PARAMETER_INVALID)
    );
}

/// Settings of the previous object leaked into the next one. A reader keeps nothing between files: the second file
/// is read as if the first had never existed, even if the first left the reader in a negative image, incremental
/// coordinates, inches, a clear polarity, an aperture selected and an open block.
#[test]
fn nothing_of_a_file_is_kept_for_the_next_one() {
    let messy = "%FSLIX24Y24*%\n%MOIN*%\n%IPNEG*%\n%OFA5B5*%\n%ADD10C,0.1*%\n%LPC*%\n%LMX*%\n%ABD11*%\nD10*\nG91*\nG74*\nG36*\nX1Y1D02*\n";
    let clean = "%FSLAX35Y35*%\n%MOMM*%\n%ADD10R,2X1*%\nD10*\nX100000Y100000D03*\nM02*\n";
    let alone = normalize(&parse(clean));
    for _ in 0..3 {
        let _ = parse(messy);
        assert_eq!(normalize(&parse(clean)), alone);
    }
    let after = parse(clean);
    assert!(after.is_clean(), "{:?}", after.diagnostics);
    assert!(!after.image.negative && after.image.is_identity());
    // and the same text twice is the same reading
    assert_eq!(normalize(&parse(messy)), normalize(&parse(messy)));
}

/// A reading that never ends (a process that does not stop when it is asked to, because it waits for something that
/// never comes). The work of a reading is bounded by the size of its input and by its limits: a file with millions of
/// blocks, of nested blocks or of step repeats ends with what it has and says why.
#[test]
fn a_hostile_file_ends_with_a_named_reason_and_what_was_read() {
    let mut text = String::from("%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\n");
    for i in 0..300_000 {
        text.push_str(&format!("X{}Y0D03*\n", i % 90_000));
    }
    let all = parse(&text);
    assert_eq!(all.commands.len(), 300_001);
    let cut = parse_with_limits(
        &text,
        &ParseLimits {
            max_commands: 1000,
            ..ParseLimits::default()
        },
    );
    assert_eq!(cut.commands.len(), 1000);
    assert!(cut
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
    // a step and repeat that would make a million million copies is refused when it is read, and what the resolver makes is bounded too
    let sr = parse(
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,1*%\nD10*\n%SRX99999Y99999I1J1*%\nX0Y0D03*\n%SR*%\nM02*\n",
    );
    assert!(sr
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
    assert_eq!(resolve(&sr).counts().0, 1);
    let many = parse(
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,1*%\nD10*\n%SRX900Y900I1J1*%\nX0Y0D03*\n%SR*%\nM02*\n",
    );
    let image = resolve_with(
        &many,
        &ResolveLimits {
            max_items: 5000,
            ..ResolveLimits::default()
        },
    );
    assert_eq!(image.items.len(), 5000);
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::IMAGE_TOO_LARGE));
}

/// A save that failed with "circular reference" after a job of several depths. The structures of a reading refer to
/// nothing by pointer: a block aperture that draws itself, a chain of blocks, and a block inside a repeat that holds
/// the block are all finite, can be cloned, compared and printed, and the resolver names the cycle instead of
/// following it.
#[test]
fn a_block_that_refers_to_itself_is_finite_and_the_cycle_is_named() {
    let text = format!(
        "{HEAD}%ADD10C,1*%\n%ABD11*%\nD10*\nX0Y0D03*\nD12*\nX10000Y0D03*\n%AB*%\n%ABD12*%\nD11*\nX0Y0D03*\n%SRX2Y2I5J5*%\nD11*\nX0Y0D03*\n%SR*%\n%AB*%\nD12*\nX0Y0D03*\nM02*\n"
    );
    let file = parse(&text);
    let copy = file.clone();
    assert_eq!(copy, file);
    assert!(!format!("{file:?}").is_empty());
    let image = resolve(&file);
    let again = resolve(&copy);
    assert_eq!(image, again);
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_CYCLE));
    assert!(!format!("{image:?}").is_empty());
    assert_eq!(image.clone(), image);
}

/// A size of zero ended in an exception instead of a clean answer ("cut Z of 0"). Zero sizes in a Gerber file are
/// data, not errors: an aperture of no size, a line of no length, an arc of no radius and a macro with a zero
/// divisor must give a reading, a named notice and no panic.
#[test]
fn a_size_of_zero_is_a_clean_reading_and_never_a_panic() {
    let file = parse(&format!(
        "{HEAD}%ADD10C,0*%\n%ADD11R,0X0*%\n%ADD12P,0X5*%\n%AMZERO*\n1,1,1/0,0,0*\n21,1,0,0,0,0,0*%\n%ADD13ZERO*%\nD10*\nX5Y5D02*\nX5Y5D01*\nG75*\nG03*\nX5Y5I0J0D01*\nD11*\nX5Y5D03*\nD12*\nX5Y5D03*\nD13*\nX5Y5D03*\nM02*\n"
    ));
    let image = resolve(&file);
    let snapshot = normalize(&file);
    assert!(!snapshot.is_empty());
    // the arc without a radius is said and left out; the rest is kept
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::ARC_CENTRE_MISSING));
    assert!(image.bounds(&file).is_some());
    let m = &file.macros["ZERO"];
    let evaluated = eh3d_parser_gerber::macros::evaluate(m, &[]);
    assert!(!evaluated.is_complete(), "the division by zero is listed");
    assert_eq!(
        evaluated.primitives.len(),
        1,
        "the rectangle of no size is a primitive"
    );
}

/// A pen of no width was not drawn at all (an outline drawn that way vanished). It is kept and counted.
#[test]
fn a_pen_of_no_width_is_kept_and_counted_not_dropped() {
    let file = parse(&format!(
        "{HEAD}%ADD10C,0*%\nD10*\nX0Y0D02*\nX100000Y0D01*\nX100000Y100000D01*\nX0Y100000D01*\nX0Y0D01*\nM02*\n"
    ));
    let image = resolve(&file);
    assert_eq!(image.counts().1, 4);
    assert_eq!(image.hairlines(&file), 4);
    let b = image.bounds(&file).unwrap();
    assert_eq!((b[0], b[1], b[2], b[3]), (0.0, 0.0, 10.0, 10.0));
    assert!(image
        .items
        .iter()
        .all(|i| matches!(i.shape, Shape::Stroke { .. })));
}

/// A drawing whose precision got worse far from the centre. Coordinates are decoded from whole numbers, so the
/// largest coordinate the format can hold is read to the last digit, and the integer form of the snapshot is exact.
#[test]
fn the_precision_does_not_depend_on_the_distance_from_the_centre() {
    let format = CoordinateFormat {
        zero_omission: ZeroOmission::Leading,
        integer_digits: 6,
        decimal_digits: 6,
    };
    for (digits, want) in [
        ("999999999999", 999_999_999_999_i64),
        ("-999999999999", -999_999_999_999),
        ("1", 1),
        ("500000000001", 500_000_000_001),
    ] {
        let v = decode_coordinate(digits, format).unwrap();
        assert_eq!(micro(v), want, "{digits}");
    }
    // a line far from the origin has the length it was written with, to the last digit
    let far = parse("%FSLAX66Y66*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX900000000000Y900000000000D02*\nX900000010000Y900000000000D01*\nM02*\n");
    let image = resolve(&far);
    match &image.items[0].shape {
        Shape::Stroke { from, to, .. } => assert_eq!(micro(to.0 - from.0), 10_000),
        other => panic!("{other:?}"),
    }
    let text = normalize(&far);
    assert!(
        text.contains("bounds: 899999950000 899999950000 900000060000 900000050000"),
        "{text}"
    );
}
