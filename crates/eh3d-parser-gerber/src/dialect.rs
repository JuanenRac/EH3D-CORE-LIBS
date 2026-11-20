//! The program that wrote a file, and what a file name says about its layer.
//!
//! The Gerber files of the CAD programs differ in their headers, in the comments they leave and in the
//! extensions of their names. [`detect`] recognises Altium, Allegro, EAGLE and KiCad by the first lines of
//! the file, and [`layer_from_filename`] reads the layer of a file from its name by the conventions of those
//! four programs. Both are guesses to help the person: the reading of the file does not depend on them and the
//! attributes of X2 (`.FileFunction`), when the file has them, are the better source.

pub use eh3d_parser_common::dialect::{detect, Dialect};

/// The side of the board a layer is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerSide {
    /// The top side.
    Top,
    /// The bottom side.
    Bottom,
    /// An inner layer, numbered from 1.
    Inner(u8),
    /// Not a side (the outline, a drill file).
    None,
}

/// What a layer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    /// Copper.
    Copper,
    /// Solder mask.
    SolderMask,
    /// Silkscreen.
    Legend,
    /// Solder paste.
    Paste,
    /// The outline of the board.
    Profile,
    /// A drill file with holes of any kind.
    Drill,
    /// A drill file of plated holes.
    DrillPlated,
    /// A drill file of holes that are not plated.
    DrillNonPlated,
    /// The job file (`.gbrjob`).
    Job,
}

/// What the name of a file says about its layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerGuess {
    /// The program whose convention matched.
    pub dialect: Dialect,
    /// What the layer is.
    pub kind: LayerKind,
    /// The side.
    pub side: LayerSide,
}

fn guess(dialect: Dialect, kind: LayerKind, side: LayerSide) -> Option<LayerGuess> {
    Some(LayerGuess {
        dialect,
        kind,
        side,
    })
}

/// The layer a file name tells, or `None` when the name follows no convention of the four programs. The
/// case of the name does not matter; the folder is ignored.
pub fn layer_from_filename(path: &str) -> Option<LayerGuess> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let lower = name.to_ascii_lowercase();
    // KiCad: <board>-F_Cu.gbr, -B_Mask.gbr, -Edge_Cuts.gbr, -In1_Cu.gbr, -PTH.drl, -NPTH.drl, .gbrjob
    if lower.ends_with(".gbrjob") {
        return guess(Dialect::KiCad, LayerKind::Job, LayerSide::None);
    }
    let stem = lower.rsplit_once('.').map_or(lower.as_str(), |(s, _)| s);
    let ext = lower.rsplit_once('.').map_or("", |(_, e)| e);
    if let Some((_, tail)) = stem.rsplit_once('-') {
        let kicad = match tail {
            "f_cu" => Some((LayerKind::Copper, LayerSide::Top)),
            "b_cu" => Some((LayerKind::Copper, LayerSide::Bottom)),
            "f_mask" => Some((LayerKind::SolderMask, LayerSide::Top)),
            "b_mask" => Some((LayerKind::SolderMask, LayerSide::Bottom)),
            "f_silkscreen" | "f_silks" => Some((LayerKind::Legend, LayerSide::Top)),
            "b_silkscreen" | "b_silks" => Some((LayerKind::Legend, LayerSide::Bottom)),
            "f_paste" => Some((LayerKind::Paste, LayerSide::Top)),
            "b_paste" => Some((LayerKind::Paste, LayerSide::Bottom)),
            "edge_cuts" => Some((LayerKind::Profile, LayerSide::None)),
            "pth" if ext == "drl" => Some((LayerKind::DrillPlated, LayerSide::None)),
            "npth" if ext == "drl" => Some((LayerKind::DrillNonPlated, LayerSide::None)),
            t => t
                .strip_prefix("in")
                .and_then(|r| r.strip_suffix("_cu"))
                .and_then(|n| n.parse::<u8>().ok())
                .map(|n| (LayerKind::Copper, LayerSide::Inner(n))),
        };
        if let Some((kind, side)) = kicad {
            return guess(Dialect::KiCad, kind, side);
        }
    }
    // Altium and the Protel family: .GTL, .GBL, .GTS, .GBS, .GTO, .GBO, .GTP, .GBP, .GKO, .GM1, .GML, .G1, .GP1
    let altium = match ext {
        "gtl" => Some((LayerKind::Copper, LayerSide::Top)),
        "gbl" => Some((LayerKind::Copper, LayerSide::Bottom)),
        "gts" => Some((LayerKind::SolderMask, LayerSide::Top)),
        "gbs" => Some((LayerKind::SolderMask, LayerSide::Bottom)),
        "gto" => Some((LayerKind::Legend, LayerSide::Top)),
        "gbo" => Some((LayerKind::Legend, LayerSide::Bottom)),
        "gtp" => Some((LayerKind::Paste, LayerSide::Top)),
        "gbp" => Some((LayerKind::Paste, LayerSide::Bottom)),
        "gko" | "gm1" | "gml" => Some((LayerKind::Profile, LayerSide::None)),
        e => e
            .strip_prefix("gp")
            .or_else(|| e.strip_prefix('g'))
            .and_then(|n| n.parse::<u8>().ok())
            .filter(|n| (1..=30).contains(n))
            .map(|n| (LayerKind::Copper, LayerSide::Inner(n))),
    };
    if let Some((kind, side)) = altium {
        return guess(Dialect::Altium, kind, side);
    }
    // EAGLE: .cmp .sol .stc .sts .plc .pls .crc .crs .mil .dim, drill .drd
    let eagle = match ext {
        "cmp" => Some((LayerKind::Copper, LayerSide::Top)),
        "sol" => Some((LayerKind::Copper, LayerSide::Bottom)),
        "stc" => Some((LayerKind::SolderMask, LayerSide::Top)),
        "sts" => Some((LayerKind::SolderMask, LayerSide::Bottom)),
        "plc" => Some((LayerKind::Legend, LayerSide::Top)),
        "pls" => Some((LayerKind::Legend, LayerSide::Bottom)),
        "crc" => Some((LayerKind::Paste, LayerSide::Top)),
        "crs" => Some((LayerKind::Paste, LayerSide::Bottom)),
        "mil" | "dim" => Some((LayerKind::Profile, LayerSide::None)),
        "drd" => Some((LayerKind::Drill, LayerSide::None)),
        e => e
            .strip_prefix("ly")
            .and_then(|n| n.parse::<u8>().ok())
            .filter(|n| (2..=15).contains(n))
            .map(|n| (LayerKind::Copper, LayerSide::Inner(n - 1))),
    };
    if let Some((kind, side)) = eagle {
        return guess(Dialect::Eagle, kind, side);
    }
    // Allegro writes .art files whose names say the layer: top.art, bottom.art, silkscreen_top.art, ...
    if ext == "art" {
        let top = stem.contains("top");
        let bottom = stem.contains("bot");
        let side = if top {
            LayerSide::Top
        } else if bottom {
            LayerSide::Bottom
        } else {
            LayerSide::None
        };
        let kind = if stem.contains("silk") || stem.contains("legend") {
            LayerKind::Legend
        } else if stem.contains("mask") {
            LayerKind::SolderMask
        } else if stem.contains("paste") {
            LayerKind::Paste
        } else if stem.contains("outline") || stem.contains("profile") || stem.contains("board") {
            LayerKind::Profile
        } else if top || bottom || stem.starts_with('l') {
            LayerKind::Copper
        } else {
            return None;
        };
        return guess(Dialect::Allegro, kind, side);
    }
    None
}

/// True when the name looks like a drill file of the programs (`.drl`, `.xln`, `.drd`, `.txt` with "drill"
/// in the name, `.tap`).
pub fn is_drill_filename(path: &str) -> bool {
    let name = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .to_ascii_lowercase();
    let ext = name.rsplit_once('.').map_or("", |(_, e)| e);
    matches!(ext, "drl" | "xln" | "drd" | "tap" | "exc" | "ncd")
        || (ext == "txt" && (name.contains("drill") || name.contains("nc")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layer_comes_from_the_name_by_each_convention() {
        let g = |n: &str| layer_from_filename(n).map(|l| (l.dialect, l.kind, l.side));
        assert_eq!(
            g("board.GTL"),
            Some((Dialect::Altium, LayerKind::Copper, LayerSide::Top))
        );
        assert_eq!(
            g("C:\\jobs\\board.G2"),
            Some((Dialect::Altium, LayerKind::Copper, LayerSide::Inner(2)))
        );
        assert_eq!(
            g("demo-F_Cu.gbr"),
            Some((Dialect::KiCad, LayerKind::Copper, LayerSide::Top))
        );
        assert_eq!(
            g("demo-In3_Cu.gbr"),
            Some((Dialect::KiCad, LayerKind::Copper, LayerSide::Inner(3)))
        );
        assert_eq!(
            g("demo-Edge_Cuts.gbr"),
            Some((Dialect::KiCad, LayerKind::Profile, LayerSide::None))
        );
        assert_eq!(
            g("demo-NPTH.drl"),
            Some((Dialect::KiCad, LayerKind::DrillNonPlated, LayerSide::None))
        );
        assert_eq!(
            g("demo.gbrjob"),
            Some((Dialect::KiCad, LayerKind::Job, LayerSide::None))
        );
        assert_eq!(
            g("board.sol"),
            Some((Dialect::Eagle, LayerKind::Copper, LayerSide::Bottom))
        );
        assert_eq!(
            g("board.PLC"),
            Some((Dialect::Eagle, LayerKind::Legend, LayerSide::Top))
        );
        assert_eq!(
            g("board.ly2"),
            Some((Dialect::Eagle, LayerKind::Copper, LayerSide::Inner(1)))
        );
        assert_eq!(
            g("silkscreen_top.art"),
            Some((Dialect::Allegro, LayerKind::Legend, LayerSide::Top))
        );
        assert_eq!(
            g("bottom.art"),
            Some((Dialect::Allegro, LayerKind::Copper, LayerSide::Bottom))
        );
        assert_eq!(g("readme.txt"), None);
        assert_eq!(g(""), None);
        assert_eq!(g("board.g99"), None);
        assert!(is_drill_filename("x/board.DRL") && is_drill_filename("board.xln"));
        assert!(is_drill_filename("board_drill.txt") && !is_drill_filename("board.gtl"));
    }
}
