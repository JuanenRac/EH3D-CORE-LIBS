//! The drill files that come from a drilling program, as a twin of that program writes them, and the damage a file can suffer on its way to the reader: cut short,
//! text after the end, a tool that is not defined, bytes that are not UTF-8, lines ended another way, bytes flipped. The reader must never panic, must say what is wrong
//! with a stable code, and the file written again from the model must be clean. The jobs come from a seeded generator, so every run is the same. Nothing here has
//! been opened by a machine.

use eh3d_parser_excellon::{parse, parse_bytes, ExcellonFile, Hit, Point};
use eh3d_writer_excellon::{write, ExcellonOptions};

/// A small deterministic generator (SplitMix64): the same seed always gives the same numbers.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// A job of `count` holes with three tools, every tool used at least once.
fn job(seed: u64, count: usize) -> ExcellonFile {
    let mut rng = Seeded(seed);
    let mut file = parse("M48\nMETRIC,TZ\nT01C0.8\nT02C1\nT05C3.175\n%\nM30\n");
    for i in 0..count.max(3) {
        let tool = if i < 3 {
            [1, 2, 5][i]
        } else {
            [1, 2, 5][rng.below(3) as usize]
        };
        file.drills.push(Hit {
            line: 0,
            tool,
            at: Point {
                x: rng.below(400_000) as f64 / 1000.0,
                y: rng.below(300_000) as f64 / 1000.0,
            },
        });
    }
    file
}

fn written(seed: u64, count: usize) -> String {
    write(&job(seed, count), &ExcellonOptions::default()).unwrap()
}

fn codes(file: &ExcellonFile) -> Vec<&'static str> {
    file.diagnostics.iter().map(|d| d.code).collect()
}

fn holes(file: &ExcellonFile) -> Vec<(u32, u64, u64)> {
    let mut v: Vec<(u32, u64, u64)> = file
        .drills
        .iter()
        .map(|h| (h.tool, h.at.x.to_bits(), h.at.y.to_bits()))
        .collect();
    v.sort_unstable();
    v
}

#[test]
fn the_same_seed_gives_the_same_file_and_the_file_reads_clean() {
    assert_eq!(written(7, 30), written(7, 30));
    assert_ne!(written(7, 30), written(8, 30));
    for seed in 0..10 {
        let file = parse(&written(seed, 25));
        assert!(file.is_clean(), "seed {seed}: {:?}", file.diagnostics);
        assert!(file.ended);
        assert_eq!(file.drills.len(), 25);
    }
}

#[test]
fn a_file_cut_short_says_it_does_not_end_and_keeps_its_holes() {
    let text = written(3, 20);
    let cut = text.strip_suffix("M30\n").expect("the file ends with M30");
    let file = parse(cut);
    assert!(!file.ended);
    assert!(codes(&file).contains(&"end-missing"));
    assert_eq!(file.drills.len(), 20);
}

#[test]
fn text_after_the_end_is_reported_and_ignored() {
    let text = format!("{}X1.0Y1.0\n", written(3, 20));
    let file = parse(&text);
    assert!(codes(&file).contains(&"text-after-end"));
    assert_eq!(file.drills.len(), 20);
}

#[test]
fn a_tool_that_is_not_defined_is_an_error_and_its_holes_are_not_kept() {
    let text = written(4, 30);
    let without: String = text
        .lines()
        .filter(|l| !l.starts_with("T02C"))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_ne!(without, text, "the definition of T02 was not found");
    let file = parse(&without);
    assert!(!file.is_clean());
    assert!(codes(&file).contains(&"tool-undefined"));
    let kept = job(4, 30).drills.iter().filter(|h| h.tool != 2).count();
    assert_eq!(file.drills.len(), kept);
}

#[test]
fn bytes_that_are_not_utf8_are_read_as_latin_1_with_a_warning() {
    let mut bytes = written(5, 12).into_bytes();
    // a comment with an accented letter written the old way
    let at = "M48\n".len();
    for (i, b) in b";caf\xE9\n".iter().enumerate() {
        bytes.insert(at + i, *b);
    }
    let file = parse_bytes(&bytes).unwrap();
    assert!(codes(&file).contains(&"input-not-utf8"));
    assert!(
        file.is_clean(),
        "a warning is not an error: {:?}",
        file.diagnostics
    );
    assert_eq!(file.drills.len(), 12);
}

#[test]
fn another_end_of_line_gives_the_same_holes() {
    let text = written(6, 40);
    let original = parse(&text);
    let crlf = parse(&text.replace('\n', "\r\n"));
    let cr_only = parse(&text.replace('\n', "\r"));
    assert!(crlf.is_clean());
    assert_eq!(holes(&crlf), holes(&original));
    // a file with only carriage returns is one line to the reader: it must still answer, and not panic
    let _ = cr_only.diagnostics.len();
}

#[test]
fn bytes_flipped_at_random_never_panic_and_always_give_the_same_answer() {
    let good = written(9, 25).into_bytes();
    let mut rng = Seeded(31337);
    for _ in 0..400 {
        let mut bytes = good.clone();
        for _ in 0..1 + rng.below(3) {
            let at = rng.below(bytes.len() as u64) as usize;
            bytes[at] = rng.below(256) as u8;
        }
        let first = parse_bytes(&bytes).map(|f| (f.drills.len(), codes(&f)));
        let second = parse_bytes(&bytes).map(|f| (f.drills.len(), codes(&f)));
        assert_eq!(first, second);
    }
}

#[test]
fn writing_the_model_again_is_the_way_back_from_every_damage() {
    let model = job(11, 35);
    let text = write(&model, &ExcellonOptions::default()).unwrap();
    let damaged = text.replace("M30\n", "");
    assert!(!parse(&damaged).ended);
    // the model is what was kept: writing it again gives the same clean file, byte for byte
    let again = write(&model, &ExcellonOptions::default()).unwrap();
    assert_eq!(again, text);
    let file = parse(&again);
    assert!(file.is_clean() && file.ended);
    assert_eq!(holes(&file), holes(&model));
}
