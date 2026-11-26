//! What the writer gives for files that were damaged: either an error or a file that the reader reads back with the same holes,
//! slots and routes, and that is written the same way again. The damage comes from a fixed rule.

mod support;

use eh3d_parser_excellon::{parse, ExcellonFile, Tool};
use eh3d_writer_excellon::{write, CoordinateStyle, ExcellonOptions, WriteError};

fn counts(file: &ExcellonFile) -> (usize, usize, Vec<usize>) {
    (
        file.drills.len(),
        file.slots.len(),
        file.routes.iter().map(|r| r.points.len()).collect(),
    )
}

#[test]
fn a_damaged_file_is_refused_or_written_as_the_reader_reads_it() {
    let sources = support::sources(&["drl", "txt", "drd"]);
    assert!(sources.len() > 8, "{}", sources.len());
    let styles = [
        CoordinateStyle::Decimal { decimals: 6 },
        CoordinateStyle::Fixed {
            integer_digits: 4,
            decimal_digits: 4,
        },
    ];
    let mut random = support::Random(0x5EED_0006);
    for (name, text) in &sources {
        for round in 0..300 {
            let damaged = support::damaged(text, &mut random);
            let model = parse(&damaged);
            for style in styles {
                let options = ExcellonOptions {
                    style,
                    ..ExcellonOptions::default()
                };
                let Ok(written) = write(&model, &options) else {
                    continue;
                };
                let back = parse(&written);
                assert_eq!(
                    counts(&back),
                    counts(&model),
                    "{name} {round} {style:?}\n{damaged}\n---\n{written}"
                );
                assert_eq!(
                    write(&back, &options).as_deref(),
                    Ok(written.as_str()),
                    "{name} {round} {style:?}\n{damaged}"
                );
            }
        }
    }
}

#[test]
fn a_tool_too_small_to_be_written_is_an_error() {
    let mut file = parse("M48\nMETRIC,TZ\nT01C1\n%\nT01\nX1.0Y1.0\nM30\n");
    file.tools.insert(
        1,
        Tool {
            number: 1,
            diameter: 1e-9,
            feed: None,
            speed: None,
        },
    );
    assert_eq!(
        write(&file, &ExcellonOptions::default()),
        Err(WriteError::BadTool(1))
    );
}
