//! The codes of a program that stay in force until another of their group changes them, as far as they can be given again after a stop: the unit, the distance mode, the plane,
//! the work coordinate system, the feed mode, the feed, the spindle with its speed and the coolant. A program that is taken up again from the middle starts by giving them back
//! ([`ModalState::restore_lines`]); the position is not part of this (see [`crate::interpret`]).

use crate::Block;

/// The modal codes that were in force at a point of a program. Every group is `None` until the program sets it.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ModalState {
    /// `G20` or `G21` as ten times the number (200 or 210).
    pub units: Option<i32>,
    /// `G90` or `G91` as ten times the number.
    pub distance: Option<i32>,
    /// `G17`, `G18` or `G19` as ten times the number.
    pub plane: Option<i32>,
    /// `G54` to `G59` as ten times the number (`G54` is 540).
    pub wcs: Option<i32>,
    /// `G94` or `G95` as ten times the number.
    pub feed_mode: Option<i32>,
    /// The last `F`.
    pub feed: Option<f64>,
    /// The last of `M3`, `M4`, `M5`.
    pub spindle: Option<i32>,
    /// The last `S`.
    pub speed: Option<f64>,
    /// The last of `M7`, `M8`, `M9`.
    pub coolant: Option<i32>,
}

impl ModalState {
    /// Takes the words of a line (a letter and its number, in the order they were written) into the state.
    pub fn apply_words(&mut self, words: impl IntoIterator<Item = (char, f64)>) {
        for (letter, v) in words {
            match letter {
                'G' => {
                    let g = (v * 10.0).round() as i32;
                    match g {
                        200 | 210 => self.units = Some(g),
                        900 | 910 => self.distance = Some(g),
                        170 | 180 | 190 => self.plane = Some(g),
                        540 | 550 | 560 | 570 | 580 | 590 => self.wcs = Some(g),
                        940 | 950 => self.feed_mode = Some(g),
                        _ => {}
                    }
                }
                'M' => match v.round() as i32 {
                    m @ 3..=5 => self.spindle = Some(m),
                    m @ 7..=9 => self.coolant = Some(m),
                    _ => {}
                },
                'F' => self.feed = Some(v),
                'S' => self.speed = Some(v),
                _ => {}
            }
        }
    }

    /// Takes a block of a program read by this crate into the state.
    pub fn apply_block(&mut self, block: &Block) {
        self.apply_words(block.words.iter().map(|w| (w.letter, w.value)));
    }

    /// The lines that give the state back. The spindle and the coolant only when `outputs` says so: starting a spindle is not something to do by itself.
    pub fn restore_lines(&self, outputs: bool) -> Vec<String> {
        let mut out = Vec::new();
        let groups: Vec<String> = [
            self.units,
            self.distance,
            self.plane,
            self.wcs,
            self.feed_mode,
        ]
        .iter()
        .flatten()
        .map(|g| {
            if g % 10 == 0 {
                format!("G{}", g / 10)
            } else {
                format!("G{}.{}", g / 10, g % 10)
            }
        })
        .collect();
        if !groups.is_empty() {
            out.push(groups.join(" "));
        }
        if let Some(f) = self.feed {
            out.push(format!("F{}", short(f)));
        }
        if outputs {
            match (self.spindle, self.speed) {
                (Some(m @ (3 | 4)), Some(s)) => out.push(format!("M{m} S{}", short(s))),
                (Some(m @ (3 | 4)), None) => out.push(format!("M{m}")),
                _ => {}
            }
            if let Some(m @ (7 | 8)) = self.coolant {
                out.push(format!("M{m}"));
            }
        }
        out
    }
}

/// A number as short as it can be written (four decimals at the most).
fn short(v: f64) -> String {
    let text = format!("{v:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse, Dialect};

    fn state_of(text: &str) -> ModalState {
        let mut s = ModalState::default();
        for b in &parse(text, Dialect::Grbl).blocks {
            s.apply_block(b);
        }
        s
    }

    #[test]
    fn the_last_code_of_each_group_stays_and_the_others_are_left_out() {
        let s = state_of("G21 G90\nG17\nG54\nG94\nM3 S12000\nM8\nG1 X5 F300\nG20\nG55\nG0 X0\nM5");
        assert_eq!(
            (s.units, s.distance, s.plane, s.wcs, s.feed_mode),
            (Some(200), Some(900), Some(170), Some(550), Some(940))
        );
        assert_eq!(
            (s.feed, s.speed, s.spindle, s.coolant),
            (Some(300.0), Some(12000.0), Some(5), Some(8))
        );
    }

    #[test]
    fn the_state_is_given_back_without_starting_the_spindle_unless_asked() {
        let s = state_of("G21 G90 G55\nM3 S12000.5\nM8\nF300.25");
        assert_eq!(s.restore_lines(false), vec!["G21 G90 G55", "F300.25"]);
        assert_eq!(
            s.restore_lines(true),
            vec!["G21 G90 G55", "F300.25", "M3 S12000.5", "M8"]
        );
        assert!(ModalState::default().restore_lines(true).is_empty());
        // a spindle that is off or a coolant that is off is not given back
        assert!(state_of("M3 S10\nM5\nM8\nM9")
            .restore_lines(true)
            .is_empty());
    }

    /// A small generator with a fixed seed: the same programs on every run.
    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) % n as u64) as usize
        }
    }

    #[test]
    fn what_is_given_back_puts_the_same_state_in_force_again() {
        const LINES: &[&str] = &[
            "G20",
            "G21",
            "G90",
            "G91",
            "G17",
            "G18",
            "G19",
            "G54",
            "G55",
            "G56",
            "G57",
            "G58",
            "G59",
            "G94",
            "G95",
            "M3 S1000",
            "M4 S2500.5",
            "M5",
            "M7",
            "M8",
            "M9",
            "F100",
            "F0.25",
            "S18000",
            "G1 X5 Y5",
            "G0 Z5",
            "(note)",
            "; x",
            "G4 P1",
        ];
        let mut rng = Rng(0x00C0_FFEE);
        for _ in 0..2_000 {
            let n = 1 + rng.below(12);
            let text: Vec<&str> = (0..n).map(|_| LINES[rng.below(LINES.len())]).collect();
            let first = state_of(&text.join(
                "
",
            ));
            let given = first.restore_lines(true);
            let again = state_of(&given.join(
                "
",
            ));
            // the second state gives back exactly the same lines (a spindle that was stopped is not part of what is given back)
            assert_eq!(again.restore_lines(true), given, "{text:?}");
            // and without the outputs nothing starts a spindle or a coolant
            assert!(
                first
                    .restore_lines(false)
                    .iter()
                    .all(|l| !l.starts_with('M')),
                "{text:?}"
            );
        }
    }
}
