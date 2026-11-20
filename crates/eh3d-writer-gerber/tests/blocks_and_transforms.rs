//! Block apertures and the transforms of loaded apertures are written and read back as the same file.

use eh3d_parser_gerber::{parse, Aperture, Operation};
use eh3d_writer_gerber::{write, GerberOptions, WriteError};

const SOURCE: &str = "%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.1*%\n%ADD11R,1X1*%\n%ABD12*%\nD10*\nX0Y0D02*\nX10000000Y0D01*\nD11*\nX5000000Y5000000D03*\n%AB*%\n%ABD13*%\nD12*\nX0Y0D03*\nX1000000Y0D03*\n%AB*%\n%LMX*%\n%LR90*%\n%LS2*%\nD13*\nX100000000Y100000000D03*\n%LMN*%\n%LR0*%\n%LS1*%\nD12*\nX0Y0D03*\nM02*\n";

fn ops(text: &str) -> Vec<Operation> {
    parse(text).commands.into_iter().map(|c| c.op).collect()
}

#[test]
fn blocks_and_transforms_survive_a_round_trip() {
    let file = parse(SOURCE);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let out = write(&file, &GerberOptions::default()).unwrap();
    let back = parse(&out.text);
    assert!(back.is_clean(), "{:?}\n{}", back.diagnostics, out.text);
    assert_eq!(ops(SOURCE), ops(&out.text));
    assert_eq!(file.apertures.len(), back.apertures.len());
    for (n, aperture) in &file.apertures {
        match (aperture, &back.apertures[n]) {
            // the lines of the commands of a block are the lines of the file they were read from
            (Aperture::Block { commands: a }, Aperture::Block { commands: b }) => {
                let ops = |c: &[eh3d_parser_gerber::Command]| {
                    c.iter().map(|c| c.op.clone()).collect::<Vec<_>>()
                };
                assert_eq!(ops(a), ops(b), "D{n}");
            }
            (a, b) => assert_eq!(a, b, "D{n}"),
        }
    }
    assert!(matches!(
        back.apertures.get(&13),
        Some(Aperture::Block { .. })
    ));
    // D13 uses D12, so D12 is written first
    assert!(out.text.find("%ABD12*%").unwrap() < out.text.find("%ABD13*%").unwrap());
    // and the text is stable: writing what was read gives the same bytes
    let again = write(&back, &GerberOptions::default()).unwrap();
    assert_eq!(out.text, again.text);
}

#[test]
fn a_block_that_needs_an_aperture_that_is_not_there_or_a_cycle_is_refused() {
    let mut file = parse(SOURCE);
    file.apertures.remove(&10);
    file.apertures.remove(&11);
    assert!(matches!(
        write(&file, &GerberOptions::default()),
        Err(WriteError::BlockOrder(_))
    ));
}

#[test]
fn a_negative_image_or_the_deprecated_parameters_are_refused_instead_of_dropped() {
    for cmd in ["IPNEG", "OFA1B1", "MIA1B0", "IR90"] {
        let file = parse(&format!("%FSLAX46Y46*%\n%MOMM*%\n%{cmd}*%\nM02*\n"));
        assert_eq!(
            write(&file, &GerberOptions::default()),
            Err(WriteError::DeprecatedImageParameters),
            "{cmd}"
        );
    }
    let plain = parse("%FSLAX46Y46*%\n%MOMM*%\n%IPPOS*%\nM02*\n");
    assert!(write(&plain, &GerberOptions::default()).is_ok());
}
