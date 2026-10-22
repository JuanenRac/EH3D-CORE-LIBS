use eh3d_parser_gerber::{
    parse, Aperture, Operation, Point, Polarity, QuadrantMode, Severity, Units, ZeroOmission,
};

const BOARD: &str = "%TF.GenerationSoftware,KiCad,Pcbnew,7.0*%
%TF.FileFunction,Copper,L1,Top*%
%FSLAX46Y46*%
G04 Gerber Fmt 4.6, Leading zero omitted, Abs format (unit mm)*
%MOMM*%
%LPD*%
G01*
%ADD10C,0.250000*%
%ADD11R,1.500000X1.000000*%
%ADD12O,2.000000X1.200000X0.300000*%
%ADD13P,1.000000X6X45.000000*%
%AMRoundRect*
0 rounded rectangle*
1,1,$1,$2,$3*%
%ADD14RoundRect,0.25X1X2*%
D10*
X100000000Y50000000D02*
X110000000Y50000000D01*
G75*
G02*
X112500000Y52500000I0J2500000D01*
G01*
D11*
X120000000Y50000000D03*
G36*
X0Y0D02*
X1000000Y0D01*
X1000000Y1000000D01*
X0Y0D01*
G37*
%LPC*%
D10*
X5000000Y5000000D02*
X6000000Y6000000D01*
M02*
";

fn p(x: f64, y: f64) -> Point {
    Point { x, y }
}

#[test]
fn a_typical_copper_layer_is_read_completely() {
    let file = parse(BOARD);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    assert!(file.ended);
    assert_eq!(file.units, Some(Units::Millimetres));
    let format = file.format.unwrap();
    assert_eq!(
        (
            format.zero_omission,
            format.integer_digits,
            format.decimal_digits
        ),
        (ZeroOmission::Leading, 4, 6)
    );
    assert_eq!(file.attributes.len(), 2);
    assert_eq!(file.attributes[1].name, ".FileFunction");
    assert_eq!(file.attributes[1].values, ["Copper", "L1", "Top"]);

    assert_eq!(file.apertures.len(), 5);
    assert_eq!(
        file.apertures[&10],
        Aperture::Circle {
            diameter: 0.25,
            hole: None
        }
    );
    assert_eq!(
        file.apertures[&11],
        Aperture::Rectangle {
            x: 1.5,
            y: 1.0,
            hole: None
        }
    );
    assert_eq!(
        file.apertures[&12],
        Aperture::Obround {
            x: 2.0,
            y: 1.2,
            hole: Some(0.3)
        }
    );
    assert_eq!(
        file.apertures[&13],
        Aperture::Polygon {
            diameter: 1.0,
            vertices: 6,
            rotation: 45.0,
            hole: None
        }
    );
    assert_eq!(
        file.apertures[&14],
        Aperture::Macro {
            name: "RoundRect".into(),
            params: vec![0.25, 1.0, 2.0]
        }
    );
    assert_eq!(file.macros["RoundRect"].body.len(), 2);

    let ops: Vec<&Operation> = file.commands.iter().map(|c| &c.op).collect();
    assert_eq!(ops[0], &Operation::Polarity(Polarity::Dark));
    assert_eq!(ops[1], &Operation::SelectAperture(10));
    assert_eq!(ops[2], &Operation::Move { to: p(100.0, 50.0) });
    assert_eq!(
        ops[3],
        &Operation::Line {
            from: p(100.0, 50.0),
            to: p(110.0, 50.0)
        }
    );
    assert_eq!(
        ops[4],
        &Operation::Arc {
            from: p(110.0, 50.0),
            to: p(112.5, 52.5),
            center_offset: (0.0, 2.5),
            clockwise: true,
            quadrant: QuadrantMode::Multi
        }
    );
    assert_eq!(ops[5], &Operation::SelectAperture(11));
    assert_eq!(ops[6], &Operation::Flash { at: p(120.0, 50.0) });
    assert_eq!(ops[7], &Operation::RegionStart);
    assert_eq!(
        ops[11],
        &Operation::Line {
            from: p(1.0, 1.0),
            to: p(0.0, 0.0)
        }
    );
    assert_eq!(ops[12], &Operation::RegionEnd);
    assert_eq!(ops[13], &Operation::Polarity(Polarity::Clear));
    assert_eq!(
        ops.last().unwrap(),
        &&Operation::Line {
            from: p(5.0, 5.0),
            to: p(6.0, 6.0)
        }
    );
}

#[test]
fn every_command_knows_its_line() {
    let file = parse(BOARD);
    let flash = file
        .commands
        .iter()
        .find(|c| matches!(c.op, Operation::Flash { .. }))
        .unwrap();
    let expected = BOARD
        .lines()
        .position(|l| l.starts_with("X120000000"))
        .unwrap()
        + 1;
    assert_eq!(flash.line, expected);
}

#[test]
fn omitted_coordinates_keep_their_previous_value() {
    let file = parse("%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX100000Y200000D02*\nX300000D01*\nY400000D01*\nM02*\n");
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let lines: Vec<_> = file
        .commands
        .iter()
        .filter_map(|c| {
            if let Operation::Line { from, to } = c.op {
                Some((from, to))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        lines,
        [
            (p(10.0, 20.0), p(30.0, 20.0)),
            (p(30.0, 20.0), p(30.0, 40.0))
        ]
    );
}

#[test]
fn trailing_zero_omission_pads_on_the_right_and_is_reported_as_deprecated() {
    let file = parse("%FSTAX24Y24*%\n%MOIN*%\n%ADD10C,0.01*%\nD10*\nX1Y12D02*\nX2Y12D01*\nM02*\n");
    assert_eq!(file.units, Some(Units::Inches));
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Warning && d.message.contains("trailing")));
    assert_eq!(file.commands[1].op, Operation::Move { to: p(10.0, 12.0) });
    assert_eq!(Units::Inches.to_mm(1.0), 25.4);
}

#[test]
fn a_bad_block_is_reported_and_reading_goes_on() {
    let text = "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.5*%\nD10*\nX10000Y10000D02*\nX20000Y10000D01*\nZ99*\nX30000Y10000D01*\nX4000000000Y1D01*\n%ADD11C,abc*%\nX50000Y10000D01*\n";
    let file = parse(text);
    let lines = file
        .commands
        .iter()
        .filter(|c| matches!(c.op, Operation::Line { .. }))
        .count();
    assert_eq!(lines, 3, "{:?}", file.commands);
    let messages: Vec<_> = file
        .diagnostics
        .iter()
        .map(|d| (d.line, d.severity))
        .collect();
    assert!(messages.contains(&(7, Severity::Warning)), "{messages:?}");
    assert!(messages.contains(&(9, Severity::Error)), "{messages:?}");
    assert!(messages.contains(&(10, Severity::Error)), "{messages:?}");
    assert!(file.diagnostics.iter().any(|d| d.message.contains("M02")));
    assert!(!file.is_clean());
}

#[test]
fn wrong_use_of_apertures_and_regions_is_reported() {
    let before = parse("%FSLAX24Y24*%\n%MOMM*%\nX1Y1D03*\nM02*\n");
    assert!(before
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.message.contains("before any aperture")));
    let undefined = parse("%FSLAX24Y24*%\n%MOMM*%\nD99*\nX1Y1D03*\nM02*\n");
    assert!(undefined
        .diagnostics
        .iter()
        .any(|d| d.message.contains("not defined")));
    let open_region = parse("%FSLAX24Y24*%\n%MOMM*%\nG36*\nX1Y1D02*\nM02*\n");
    assert!(open_region
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error && d.message.contains("never closed")));
    let stray = parse("%FSLAX24Y24*%\nG37*\nM02*\n");
    assert!(stray
        .diagnostics
        .iter()
        .any(|d| d.message.contains("without G36")));
    let no_start = parse("%FSLAX24Y24*%\n%ADD10C,1*%\nD10*\nX1Y1D01*\nM02*\n");
    assert!(no_start
        .diagnostics
        .iter()
        .any(|d| d.message.contains("start point")));
}

#[test]
fn a_coordinate_before_the_format_and_an_open_command_are_reported() {
    let file = parse("%MOMM*%\nX1Y1D02*\n");
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.message.contains("before the format")));
    let open = parse("%FSLAX24Y24*\n");
    assert!(open
        .diagnostics
        .iter()
        .any(|d| d.message.contains("never closed")));
}

#[test]
fn deprecated_and_unsupported_commands_do_not_stop_the_reading() {
    let file = parse("%FSLAX24Y24*%\n%MOIN*%\n%IPPOS*%\n%LMX*%\n%AB10*%\nG70*\nG54D10*\nM02*\n");
    assert!(file.ended);
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Info && d.message.contains("IP")));
    assert!(file.diagnostics.iter().any(|d| d.message.contains("LM")));
    assert!(file.diagnostics.iter().any(|d| d.message.contains("G70")));
}

#[test]
fn step_and_repeat_is_read() {
    let file = parse("%FSLAX24Y24*%\n%MOMM*%\n%SRX3Y2I10.5J20*%\n%SR*%\nM02*\n");
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    match &file.commands[0].op {
        Operation::StepRepeat(Some(sr)) => assert_eq!(
            (sr.x_repeats, sr.y_repeats, sr.x_step, sr.y_step),
            (3, 2, 10.5, 20.0)
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(file.commands[1].op, Operation::StepRepeat(None));
    assert!(parse("%FSLAX24Y24*%\n%SRX0Y1I1J1*%\n")
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error));
}

#[test]
fn incompatible_format_and_aperture_definitions_are_errors() {
    for text in [
        "%FSLIX24Y24*%",
        "%FSLAX77Y24*%",
        "%FSLAX2Y24*%",
        "%FSAAX24Y24*%",
        "%ADD5C,1*%",
        "%ADDC,1*%",
        "%ADD10P,1,2*%",
        "%ADD10R,1*%",
        "%MOXX*%",
        "%LPX*%",
    ] {
        let file = parse(&format!("{text}\n"));
        assert!(
            file.diagnostics
                .iter()
                .any(|d| d.severity == Severity::Error),
            "{text}: {:?}",
            file.diagnostics
        );
    }
}

#[test]
fn no_prefix_of_a_file_makes_the_reader_panic() {
    let chars: Vec<char> = BOARD.chars().collect();
    for end in 0..=chars.len() {
        let prefix: String = chars[..end].iter().collect();
        let _ = parse(&prefix);
    }
}

#[test]
fn garbage_never_panics() {
    let samples = [
        "",
        "\u{feff}",
        "%",
        "%%",
        "*",
        "**",
        "%*%",
        "X",
        "D",
        "G",
        "-",
        "X-Y-D01*",
        "%FS*%",
        "%AD*%",
        "%ADD*%",
        "%AM*%",
        "ñññ*\u{1F600}*",
        "\0\0\0",
        "%TF*%",
        "%SR",
        "G04",
        "D01*D02*D03*",
        "X1Y1X2Y2D01D02*",
    ];
    for sample in samples {
        let _ = parse(sample);
    }
    let mut seed: u64 = 0x1234_5678_9abc_def0;
    let alphabet: Vec<char> = "%*GDXYIJMLPAFSOT0123456789.,-+ \n\rCRO".chars().collect();
    for _ in 0..300 {
        let mut text = String::new();
        for _ in 0..200 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            text.push(alphabet[(seed >> 33) as usize % alphabet.len()]);
        }
        let _ = parse(&text);
    }
}
