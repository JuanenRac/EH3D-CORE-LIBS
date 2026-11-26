//! What the writer gives for files that were damaged: either an error or a file that the reader reads back with the same
//! operations and apertures, and that is written the same way again. The damage comes from a fixed rule.

mod support;

use eh3d_parser_gerber::{parse, Attribute};
use eh3d_writer_gerber::{write, GerberOptions, WriteError};

#[test]
fn a_damaged_file_is_refused_or_written_as_the_reader_reads_it() {
    let sources = support::sources(&["gbr", "gtl", "gbl", "gbs", "gto", "gm1", "art", "cmp"]);
    assert!(sources.len() > 12, "{}", sources.len());
    let options = GerberOptions::default();
    let mut random = support::Random(0x5EED_0007);
    for (name, text) in &sources {
        for round in 0..300 {
            let damaged = support::damaged(text, &mut random);
            let model = parse(&damaged);
            let Ok(written) = write(&model, &options) else {
                continue;
            };
            let back = parse(&written.text);
            assert_eq!(
                back.apertures.len(),
                model.apertures.len(),
                "{name} {round}\n{damaged}\n---\n{}",
                written.text
            );
            let again = write(&back, &options).map(|w| w.text);
            assert_eq!(
                again.as_ref(),
                Ok(&written.text),
                "{name} {round}\n{damaged}"
            );
        }
    }
}

#[test]
fn a_comma_inside_a_value_of_an_attribute_is_an_error() {
    let mut file = parse("%FSLAX46Y46*%\n%MOMM*%\nM02*\n");
    file.attributes.push(Attribute {
        kind: "TF".into(),
        name: ".ProjectId".into(),
        values: vec!["Box, large".into(), "id".into(), "1".into()],
    });
    assert_eq!(
        write(&file, &GerberOptions::default()),
        Err(WriteError::ForbiddenCharacter)
    );
}
