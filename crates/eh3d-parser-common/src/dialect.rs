//! The CAD program that seems to have written a Gerber or an Excellon file, from its first lines.
//!
//! It is a guess to help the person and to choose a message: the reading of a file never depends on it.

/// The program that seems to have written a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
    /// None of the ones below, or the file does not say.
    #[default]
    Generic,
    /// Altium Designer (and Protel).
    Altium,
    /// Cadence Allegro (and OrCAD PCB Editor).
    Allegro,
    /// EAGLE (CadSoft and Autodesk).
    Eagle,
    /// KiCad.
    KiCad,
}

/// How much of the start of a file is looked at.
pub const HEADER_BYTES: usize = 4096;

/// The program that wrote a text, from its first 4096 bytes (Gerber and Excellon files).
pub fn detect(text: &str) -> Dialect {
    let mut end = text.len().min(HEADER_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let head = text.get(..end).unwrap_or("").to_ascii_lowercase();
    if head.contains("kicad") || head.contains("pcbnew") || head.contains("gerbview") {
        Dialect::KiCad
    } else if head.contains("eagle") || head.contains("cadsoft") {
        Dialect::Eagle
    } else if head.contains("allegro") || head.contains("cadence") || head.contains("; holesize") {
        Dialect::Allegro
    } else if head.contains("altium")
        || head.contains("g04 layer:")
        || head.contains("protel")
        || head.contains(";layer_color")
    {
        Dialect::Altium
    } else {
        Dialect::Generic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_programs_are_told_from_gerber_and_from_drill_headers() {
        assert_eq!(
            detect("G04 #@! TF.GenerationSoftware,KiCad,Pcbnew,8.0.1*\n"),
            Dialect::KiCad
        );
        assert_eq!(
            detect("M48\n; DRILL file {KiCad 8.0} date x\n"),
            Dialect::KiCad
        );
        assert_eq!(
            detect("G04 Layer: TopLayer*\n%FSLAX24Y24*%\n"),
            Dialect::Altium
        );
        assert_eq!(detect("M48\n;Layer_Color=1\nINCH\n"), Dialect::Altium);
        assert_eq!(
            detect("G04 EAGLE Gerber RS-274X export*\n%MOMM*%\n"),
            Dialect::Eagle
        );
        assert_eq!(
            detect("G04 Cadence Allegro 17.4*\n%FSLAX25Y25*%\n"),
            Dialect::Allegro
        );
        assert_eq!(
            detect("M48\n; Holesize 1 = 0.600000 Tolerance = +0.1/-0.1 PLATED MM Quantity = 4\n"),
            Dialect::Allegro
        );
        assert_eq!(detect("%FSLAX24Y24*%\n%MOIN*%\nM02*\n"), Dialect::Generic);
        assert_eq!(detect(""), Dialect::Generic);
    }

    #[test]
    fn the_header_is_cut_at_a_character_and_a_later_mention_is_not_a_header() {
        let text = format!("{}é{}", "x".repeat(HEADER_BYTES - 1), "kicad");
        assert_eq!(detect(&text), Dialect::Generic);
        let late = format!("{}kicad", "G04 filler*\n".repeat(1000));
        assert_eq!(detect(&late), Dialect::Generic);
    }
}
