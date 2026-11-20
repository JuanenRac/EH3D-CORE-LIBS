//! The corpus of synthetic boards and the snapshots that keep every change of the readers in view.
//!
//! Each board is a small set of Gerber and Excellon files **written by this test** (no file was taken from any
//! program, project or person; each board is an original work of this repository, licensed with it under
//! GPL-3.0-or-later) in the dialect of one CAD program: Altium, Allegro, EAGLE and KiCad, and boards that exercise one
//! feature of the standard each (block apertures, step and repeat, transforms, every macro primitive, X3, negative
//! images, incremental coordinates, regions with arcs, omitted trailing zeros, a decimal comma, routes with arcs in
//! a drill file, damaged files).
//!
//! The files are in `tests/corpus/boards/<board>/`, next to the normalised text of their reading (`<file>.snap`,
//! made by `snapshot::normalize`), and `tests/corpus/manifest.json` lists every board with its dialect, its origin,
//! its licence and the SHA-256 of each file. The test writes all of them again from the code below and fails when
//! a file, a snapshot or the manifest differs from what is on disk; `EH3D_REGENERATE=1 cargo test` rewrites them so
//! the change shows in the diff. The snapshot is the safety net of every change of the readers, and of the move of
//! the other readers of the suite to this one (see `docs/MIGRATION.md`).

use eh3d_parser_gcode::emit::sha256_hex;
use std::fmt::Write as _;
use std::path::PathBuf;

/// One file of a board.
struct File {
    name: String,
    kind: Kind,
    text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Gerber,
    Drill,
}

struct Board {
    id: &'static str,
    dialect: &'static str,
    what: &'static str,
    files: Vec<File>,
}

fn gerber(name: &str, text: String) -> File {
    File {
        name: name.to_owned(),
        kind: Kind::Gerber,
        text,
    }
}

fn drill(name: &str, text: String) -> File {
    File {
        name: name.to_owned(),
        kind: Kind::Drill,
        text,
    }
}

/// A coordinate as the whole number of a format with `dec` decimals, leading zeros omitted.
fn n(v: f64, dec: u32) -> String {
    format!("{}", (v * 10f64.powi(dec as i32)).round() as i64)
}

/// `X..Y..` for a point in millimetres, in a file whose unit is `unit` (1.0 for mm, 25.4 for inches).
fn xy(x: f64, y: f64, unit: f64, dec: u32) -> String {
    format!("X{}Y{}", n(x / unit, dec), n(y / unit, dec))
}

/// The drawing every layer of the standard boards shares: a track, a pad of each kind, a region with an arc and a
/// via, all in millimetres, drawn through `d_select` (how the dialect selects an aperture) and `unit`.
struct Art {
    unit: f64,
    dec: u32,
    old_select: bool,
}

impl Art {
    fn sel(&self, d: u32) -> String {
        if self.old_select {
            format!("G54D{d}*")
        } else {
            format!("D{d}*")
        }
    }
    fn pt(&self, x: f64, y: f64) -> String {
        xy(x, y, self.unit, self.dec)
    }
    fn copper(&self) -> String {
        let mut s = String::new();
        let _ = writeln!(s, "G01*");
        let _ = writeln!(s, "{}", self.sel(10));
        let _ = writeln!(s, "{}D02*", self.pt(5.0, 5.0));
        let _ = writeln!(s, "{}D01*", self.pt(10.0, 5.0));
        let _ = writeln!(s, "{}D01*", self.pt(10.0, 12.0));
        let _ = writeln!(s, "{}", self.sel(11));
        let _ = writeln!(s, "{}D03*", self.pt(5.0, 5.0));
        let _ = writeln!(s, "{}", self.sel(12));
        let _ = writeln!(s, "{}D03*", self.pt(10.0, 12.0));
        let _ = writeln!(s, "{}", self.sel(13));
        let _ = writeln!(s, "{}D03*", self.pt(15.0, 5.0));
        let _ = writeln!(s, "G75*");
        let _ = writeln!(s, "G36*");
        let _ = writeln!(s, "{}D02*", self.pt(20.0, 8.0));
        let _ = writeln!(s, "G01*");
        let _ = writeln!(s, "{}D01*", self.pt(26.0, 8.0));
        let _ = writeln!(s, "G03*");
        let _ = writeln!(
            s,
            "{}I{}J{}D01*",
            self.pt(26.0, 14.0),
            n(0.0, self.dec),
            n(3.0 / self.unit, self.dec)
        );
        let _ = writeln!(s, "G01*");
        let _ = writeln!(s, "{}D01*", self.pt(20.0, 14.0));
        let _ = writeln!(s, "{}D01*", self.pt(20.0, 8.0));
        let _ = writeln!(s, "G37*");
        let _ = writeln!(s, "{}", self.sel(12));
        let _ = writeln!(s, "{}D03*", self.pt(25.0, 4.0));
        s
    }
    fn outline(&self) -> String {
        let mut s = String::new();
        let r = 2.0;
        let _ = writeln!(s, "G01*");
        let _ = writeln!(s, "{}", self.sel(10));
        let _ = writeln!(s, "{}D02*", self.pt(r, 0.0));
        let _ = writeln!(s, "{}D01*", self.pt(30.0 - r, 0.0));
        let _ = writeln!(s, "G75*");
        let _ = writeln!(s, "G03*");
        let i = n(0.0, self.dec);
        let j = n(r / self.unit, self.dec);
        let ri = n(r / self.unit, self.dec);
        let _ = writeln!(s, "{}I{i}J{j}D01*", self.pt(30.0, r));
        let _ = writeln!(s, "G01*");
        let _ = writeln!(s, "{}D01*", self.pt(30.0, 20.0 - r));
        let _ = writeln!(s, "G03*");
        let _ = writeln!(s, "{}I-{ri}J{i}D01*", self.pt(30.0 - r, 20.0));
        let _ = writeln!(s, "G01*");
        let _ = writeln!(s, "{}D01*", self.pt(r, 20.0));
        let _ = writeln!(s, "G03*");
        let _ = writeln!(s, "{}I{i}J-{ri}D01*", self.pt(0.0, 20.0 - r));
        let _ = writeln!(s, "G01*");
        let _ = writeln!(s, "{}D01*", self.pt(0.0, r));
        let _ = writeln!(s, "G03*");
        let _ = writeln!(s, "{}I{ri}J{i}D01*", self.pt(r, 0.0));
        s
    }
}

fn kicad() -> Board {
    let art = Art {
        unit: 1.0,
        dec: 6,
        old_select: false,
    };
    let head = |function: &str, polarity: &str| {
        format!(
            "G04 Created by KiCad (PCBNEW 8.0.1)*\n%TF.GenerationSoftware,KiCad,Pcbnew,8.0.1*%\n%TF.SameCoordinates,Original*%\n%TF.FileFunction,{function}*%\n%TF.FilePolarity,{polarity}*%\n%FSLAX46Y46*%\nG04 Gerber Fmt 4.6, Leading zero omitted, Abs format (unit mm)*\n%MOMM*%\n%LPD*%\n"
        )
    };
    let copper = format!(
        "{}%ADD10C,0.250000*%\n%TA.AperFunction,SMDPad,CuDef*%\n%ADD11R,1.600000X1.600000*%\n%ADD12C,1.600000*%\n%ADD13O,2.400000X1.400000*%\n%TD*%\n{}M02*\n",
        head("Copper,L1,Top", "Positive"),
        art.copper()
    );
    let outline = format!(
        "{}%ADD10C,0.100000*%\n{}M02*\n",
        head("Profile,NP", "Positive"),
        art.outline()
    );
    // the older programs wrote the attributes as comments
    let old = format!(
        "G04 #@! TF.GenerationSoftware,KiCad,Pcbnew,5.99*\nG04 #@! TF.FileFunction,Copper,L2,Bot*\nG04 #@! TF.FilePolarity,Positive*\n%FSLAX46Y46*%\n%MOMM*%\n%ADD10C,0.250000*%\n%ADD11R,1.600000X1.600000*%\n%ADD12C,1.600000*%\n%ADD13O,2.400000X1.400000*%\n{}M02*\n",
        art.copper()
    );
    let holes = "M48\n; DRILL file {KiCad 8.0.1} date x\n; FORMAT={-:-/ absolute / metric / decimal}\n; #@! TF.CreationDate,x\n; #@! TF.GenerationSoftware,KiCad,Pcbnew,8.0.1\n; #@! TF.FileFunction,Plated,1,2,PTH\nFMAT,2\nMETRIC\n; #@! TA.AperFunction,Plated,PTH,ViaDrill\nT1C0.300\n; #@! TA.AperFunction,Plated,PTH,ComponentDrill\nT2C0.800\n%\nG90\nG05\nT1\nX25.0Y4.0\nT2\nX5.0Y5.0\nX10.0Y12.0\nM30\n".to_owned();
    Board {
        id: "kicad",
        dialect: "KiCad",
        what: "Copper with X2 attributes, outline with arcs, copper with the attributes written as comments, and a plated drill file",
        files: vec![
            gerber("board-F_Cu.gbr", copper),
            gerber("board-Edge_Cuts.gbr", outline),
            gerber("board-B_Cu_comment_attributes.gbr", old),
            drill("board-PTH.drl", holes),
        ],
    }
}

fn altium() -> Board {
    let art = Art {
        unit: 25.4,
        dec: 4,
        old_select: true,
    };
    let copper = format!(
        "G04 Layer: TopLayer*\nG04 Created by Altium Designer*\n%FSLAX24Y24*%\n%MOIN*%\n%ADD10C,0.0100*%\n%ADD11R,0.0630X0.0630*%\n%ADD12C,0.0630*%\n%ADD13O,0.0945X0.0551*%\n%LPD*%\n{}M02*\n",
        art.copper()
    );
    let outline = format!(
        "G04 Layer: Mechanical1*\n%FSLAX24Y24*%\n%MOIN*%\n%ADD10C,0.0039*%\n%LPD*%\n{}M02*\n",
        art.outline()
    );
    let holes = "M48\n;Layer_Color=9474304\n;FILE_FORMAT=2:5\nINCH,LZ\n;TYPE=PLATED\nT1F00S00C0.01180\nT2F00S00C0.03150\n%\nT1\nX0098425Y0015748\nT2\nX0019685Y0019685\nX003937Y0047244\nM30\n".to_owned();
    let npth = "M48\n;Layer_Color=9474304\n;FILE_FORMAT=2:5\nINCH,LZ\n;TYPE=NON_PLATED\nT1F00S00C0.12600\n%\nT1\nX0005Y0005\nM30\n".to_owned();
    Board {
        id: "altium",
        dialect: "Altium",
        what: "Inch files with G54 selection, copper with a region and an arc, outline, plated and non plated drill files with leading zeros",
        files: vec![
            gerber("board.GTL", copper),
            gerber("board.GM1", outline),
            drill("board.TXT", holes),
            drill("board-NPTH.TXT", npth),
        ],
    }
}

fn eagle() -> Board {
    let art = Art {
        unit: 25.4,
        dec: 4,
        old_select: false,
    };
    let copper = format!(
        "G04 EAGLE Gerber RS-274X export*\nG75*\nG70*\n%OFA0B0*%\n%FSLAX24Y24*%\n%IPPOS*%\n%LPD*%\n%AMOC8*\n5,1,8,0,0,1.08239X$1,22.5*%\n%ADD10C,0.0100*%\n%ADD11R,0.0630X0.0630*%\n%ADD12OC8,0.0630*%\n%ADD13O,0.0945X0.0551*%\nG54D10*\n{}M02*\n",
        art.copper().replace("G54D", "D")
    );
    let outline = format!(
        "G04 EAGLE Gerber RS-274X export*\nG75*\nG70*\n%OFA0B0*%\n%FSLAX24Y24*%\n%IPPOS*%\n%LPD*%\n%ADD10C,0.0039*%\n{}M02*\n",
        art.outline()
    );
    // EAGLE writes its drill files with M71, a bare header and trailing zeros left out (TZ)
    let holes = "M48\n;EAGLE drill data\nM71\nT01C0.8\nT02C0.3\n%\nT01\nX001500Y002000\nX002500Y003500\nT02\nX005000Y010000\nM30\n".to_owned();
    Board {
        id: "eagle",
        dialect: "EAGLE",
        what: "G70 and the deprecated image parameters at their neutral value, the octagon macro of EAGLE, outline, and a drill file in the old metric form",
        files: vec![
            gerber("board.cmp", copper),
            gerber("board.mil", outline),
            drill("board.drd", holes),
        ],
    }
}

fn allegro() -> Board {
    let art = Art {
        unit: 25.4,
        dec: 5,
        old_select: true,
    };
    let copper = format!(
        "G04 Cadence Allegro 17.4*\nG04 File Origin: x*\n%FSLAX25Y25*%\n%MOIN*%\n%IN top*%\n%IPPOS*%\n%LNtop*%\n%ADD10C,.0100*%\n%ADD11R,.0630X.0630*%\n%ADD12C,.0630*%\n%ADD13O,.0945X.0551*%\n%LPD*%\n{}M02*\n",
        art.copper()
    );
    let outline = format!(
        "G04 Cadence Allegro 17.4*\n%FSLAX25Y25*%\n%MOIN*%\n%IN outline*%\n%ADD10C,.0039*%\n%LPD*%\n{}M02*\n",
        art.outline()
    );
    let holes = "M48\n; Holesize 1 = 0.300000 Tolerance = +0.050000/-0.050000 PLATED MM Quantity = 1\n; Holesize 2 = 0.800000 Tolerance = +0.100000/-0.100000 PLATED MM Quantity = 2\nMETRIC,TZ\nT1C0.300\nT2C0.800\n%\nT1\nX025000Y004000\nT2\nX005000Y005000\nX010000Y012000\nM30\n".to_owned();
    Board {
        id: "allegro",
        dialect: "Allegro",
        what: "Numbers without a leading zero (.0100), image names, 5 decimals, and a drill file with the Holesize comments",
        files: vec![
            gerber("top.art", copper),
            gerber("outline.art", outline),
            drill("drill.drl", holes),
        ],
    }
}

fn features() -> Vec<Board> {
    let head = "%FSLAX46Y46*%\n%MOMM*%\n";
    vec![
        Board {
            id: "blocks_and_repeats",
            dialect: "Generic",
            what: "A block aperture with another inside, a step and repeat, and the layer transforms LM, LR and LS",
            files: vec![gerber(
                "blocks.gbr",
                format!("{head}%ADD10C,0.1*%\n%ADD11R,1X1*%\n%ABD12*%\nD10*\nX0Y0D02*\nX10000000Y0D01*\nD11*\nX5000000Y5000000D03*\n%AB*%\n%ABD13*%\nD12*\nX0Y0D03*\n%LPC*%\nX1000000Y0D03*\n%AB*%\n%SRX3Y2I15.0J12.0*%\nD13*\nX5000000Y5000000D03*\n%SR*%\n%LMX*%\n%LR90*%\n%LS2*%\nD12*\nX60000000Y10000000D03*\n%LMN*%\n%LR0*%\n%LS1*%\nM02*\n"),
            )],
        },
        Board {
            id: "macros",
            dialect: "Generic",
            what: "A macro for each primitive of the standard, with variables and arithmetic",
            files: vec![gerber(
                "macros.gbr",
                format!("{head}%AMALL*\n0 every primitive*\n$3=$1x2+$2*\n1,1,$1,0,0*\n20,1,0.2,0,0,$3,0,0*\n21,1,$1,0.5,0,0,0*\n22,1,$1,0.5,-0.5,-0.25,0*\n4,1,3,0,0,1,0,1,1,0,0,0*\n5,1,6,0,0,$1,0*\n6,0,0,3,0.2,0.1,4,0.05,3.5,0*\n7,0,0,2,1,0.2,0*%\n%ADD10ALL,1X0.5*%\nD10*\nX0Y0D03*\nX10000000Y0D03*\nM02*\n"),
            )],
        },
        Board {
            id: "x3_components",
            dialect: "Generic",
            what: "Attributes of X2 and X3: file function, aperture functions, net, pin and component attributes",
            files: vec![gerber(
                "x3.gbr",
                format!("%TF.GenerationSoftware,Example,Maker,1.0*%\n%TF.FileFunction,Component,L1,Top*%\n%TF.FilePolarity,Positive*%\n{head}%TA.AperFunction,ComponentMain*%\n%ADD10R,3X2*%\n%TA.AperFunction,ComponentPin*%\n%ADD11C,0.4*%\n%TD*%\n%TO.C,U1*%\n%TO.CRot,90*%\n%TO.CMnt,SMD*%\n%TO.CVal,74HC00*%\nD10*\nX5000000Y5000000D03*\n%TO.P,U1,1,A*%\n%TO.N,SIG*%\nD11*\nX3500000Y5000000D03*\n%TD*%\nM02*\n"),
            )],
        },
        Board {
            id: "negative_and_image_parameters",
            dialect: "Generic",
            what: "A negative image with an offset, a scale, a mirror and a rotation of the whole image",
            files: vec![gerber(
                "negative.gbr",
                format!("{head}%IPNEG*%\n%IN Negative layer*%\n%OFA2.5B-1.5*%\n%SFA1.5B1.5*%\n%MIA1B0*%\n%IR90*%\n%ADD10C,0.5*%\nD10*\nX0Y0D02*\nX10000000Y5000000D01*\nX10000000Y5000000D03*\nM02*\n"),
            )],
        },
        Board {
            id: "incremental",
            dialect: "Generic",
            what: "Incremental coordinates by the format (FS ... I) and by G91, back to absolute with G90",
            files: vec![
                gerber(
                    "incremental_format.gbr",
                    "%FSLIX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX10000Y10000D02*\nX10000Y0D01*\nX0Y10000D01*\nX-5000D01*\nM02*\n".to_owned(),
                ),
                gerber(
                    "incremental_g91.gbr",
                    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX10000Y10000D02*\nG91*\nX10000D01*\nY10000D01*\nG90*\nX0Y0D01*\nM02*\n".to_owned(),
                ),
            ],
        },
        Board {
            id: "regions_and_arcs",
            dialect: "Generic",
            what: "Regions with arcs in multi and in single quadrant mode, a region with two contours and a full circle",
            files: vec![gerber(
                "regions.gbr",
                format!("{head}%ADD10C,0.1*%\nD10*\nG75*\nG36*\nX10000000Y0D02*\nG03*\nX0Y10000000I-10000000J0D01*\nG01*\nX0Y0D01*\nX10000000Y0D01*\nG37*\nG36*\nX20000000Y0D02*\nG01*\nX30000000Y0D01*\nX30000000Y10000000D01*\nX20000000Y0D01*\nX40000000Y0D02*\nX50000000Y0D01*\nX50000000Y10000000D01*\nX40000000Y0D01*\nG37*\nX60000000Y0D02*\nG02*\nX60000000Y0I-3000000J0D01*\nG74*\nG36*\nX0Y30000000D02*\nG03*\nX-10000000Y40000000I10000000J0D01*\nG01*\nX0Y40000000D01*\nX0Y30000000D01*\nG37*\nM02*\n"),
            )],
        },
        Board {
            id: "trailing_zeros",
            dialect: "Generic",
            what: "Coordinates with the trailing zeros omitted (deprecated) and the units given by G70",
            files: vec![gerber(
                "trailing.gbr",
                "%FSTAX24Y24*%\nG70*\n%ADD10C,0.01*%\nD10*\nX1Y1D02*\nX15Y1D01*\nX15Y25D01*\nM02*\n".to_owned(),
            )],
        },
        Board {
            id: "decimal_comma",
            dialect: "Generic",
            what: "A file written by a computer set to a language that writes the decimal point as a comma",
            files: vec![
                gerber(
                    "comma.gbr",
                    "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0,25*%\n%ADD11R,1,5X0,8*%\nD10*\nX10000Y5000D02*\nX1,5Y2,25D01*\nD11*\nX3,0Y4,0D03*\nM02*\n".to_owned(),
                ),
                drill(
                    "comma.drl",
                    "M48\nMETRIC\nT1C0,8\n%\nT1\nX12,5Y3,25\nM30\n".to_owned(),
                ),
            ],
        },
        Board {
            id: "drill_routes",
            dialect: "Generic",
            what: "Drill file with slots, a repeat, incremental coordinates and a route with arcs by radius and by offsets",
            files: vec![drill(
                "routes.drl",
                "M48\nMETRIC\nT1C0.8\nT2C2.0\n%\nT1\nX10.0Y10.0\nR3X2.0Y0.0\nX30.0Y10.0G85X34.0Y10.0\nT2\nG00X0.0Y0.0\nM15\nG01X20.0Y0.0\nG02X30.0Y10.0A10.0\nG03X20.0Y20.0I-10.0J0.0\nG01X0.0Y20.0\nM16\nG05\nG91\nT1\nX1.0Y1.0\nX1.0Y0.0\nG90\nM30\n".to_owned(),
            )],
        },
        Board {
            id: "damaged",
            dialect: "Generic",
            what: "Files with the usual damage: no end, an unclosed region, an unknown command, a cycle of block apertures, an arc with no centre",
            files: vec![
                gerber(
                    "damaged.gbr",
                    format!("{head}%ADD10C,0.1*%\n%ABD11*%\nD11*\nX0Y0D03*\n%AB*%\nD10*\nX0Y0D02*\nX5000000Y0D01*\n%XYZ123*%\nG36*\nX0Y0D02*\nX5000000Y0D01*\nD11*\nX0Y0D03*\nG75*\nG03*\nX1Y1I0J0D01*\n"),
                ),
                drill(
                    "damaged.drl",
                    "M48\nINCH\nT1C0.04\nT1C0.05\n%\nT2\nX1.0Y1.0\nT1\nG02X1.0Y1.0\nR4000000000X1.0\nX99999999999999999999\n".to_owned(),
                ),
            ],
        },
    ]
}

fn boards() -> Vec<Board> {
    let mut all = vec![kicad(), altium(), eagle(), allegro()];
    all.extend(features());
    all
}

fn corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus")
}

fn snapshot_of(file: &File) -> String {
    match file.kind {
        Kind::Gerber => {
            eh3d_parser_gerber::snapshot::normalize(&eh3d_parser_gerber::parse(&file.text))
        }
        Kind::Drill => {
            eh3d_parser_excellon::snapshot::normalize(&eh3d_parser_excellon::parse(&file.text))
        }
    }
}

fn manifest(boards: &[Board]) -> String {
    let mut out = String::from("{\n  \"description\": \"Synthetic boards written by crates/eh3d-parser-gerber/tests/corpus.rs. Original work of this repository, licensed with it. No file comes from another project.\",\n  \"license\": \"GPL-3.0-or-later\",\n  \"origin\": \"generated by the test corpus.rs of this repository\",\n  \"boards\": [\n");
    for (i, b) in boards.iter().enumerate() {
        let _ = writeln!(out, "    {{");
        let _ = writeln!(out, "      \"id\": \"{}\",", b.id);
        let _ = writeln!(out, "      \"dialect\": \"{}\",", b.dialect);
        let _ = writeln!(out, "      \"description\": \"{}\",", b.what);
        let _ = writeln!(out, "      \"license\": \"GPL-3.0-or-later\",");
        let _ = writeln!(out, "      \"origin\": \"generated by corpus.rs\",");
        let _ = writeln!(out, "      \"files\": [");
        for (j, f) in b.files.iter().enumerate() {
            let kind = match f.kind {
                Kind::Gerber => "gerber",
                Kind::Drill => "excellon",
            };
            let comma = if j + 1 < b.files.len() { "," } else { "" };
            let _ = writeln!(
                out,
                "        {{\"name\": \"{}\", \"kind\": \"{kind}\", \"bytes\": {}, \"sha256\": \"{}\"}}{comma}",
                f.name,
                f.text.len(),
                sha256_hex(f.text.as_bytes())
            );
        }
        let _ = writeln!(out, "      ]");
        let _ = writeln!(out, "    }}{}", if i + 1 < boards.len() { "," } else { "" });
    }
    out.push_str("  ]\n}\n");
    out
}

fn read(path: &PathBuf) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|t| t.replace("\r\n", "\n"))
}

#[test]
fn every_file_snapshot_and_the_manifest_are_what_the_code_writes() {
    let regenerate = std::env::var_os("EH3D_REGENERATE").is_some();
    let dir = corpus_dir();
    let all = boards();
    let mut expected: Vec<(PathBuf, String)> = Vec::new();
    for b in &all {
        for f in &b.files {
            let base = dir.join("boards").join(b.id).join(&f.name);
            expected.push((base.clone(), f.text.clone()));
            expected.push((
                PathBuf::from(format!("{}.snap", base.display())),
                snapshot_of(f),
            ));
        }
    }
    expected.push((dir.join("manifest.json"), manifest(&all)));
    if regenerate {
        for (path, text) in &expected {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        return;
    }
    for (path, text) in &expected {
        let on_disk = read(path).unwrap_or_else(|| {
            panic!(
                "{} is missing: run EH3D_REGENERATE=1 cargo test",
                path.display()
            )
        });
        assert_eq!(
            &on_disk,
            text,
            "{} differs from what the test writes",
            path.display()
        );
    }
}

#[test]
fn no_file_is_left_on_disk_that_the_corpus_does_not_write() {
    let dir = corpus_dir().join("boards");
    if !dir.is_dir() {
        return; // an installation without the corpus
    }
    let mut wanted: Vec<PathBuf> = Vec::new();
    for b in boards() {
        for f in &b.files {
            let base = dir.join(b.id).join(&f.name);
            wanted.push(base.clone());
            wanted.push(PathBuf::from(format!("{}.snap", base.display())));
        }
    }
    let mut found = Vec::new();
    for board in std::fs::read_dir(&dir).unwrap().flatten() {
        for file in std::fs::read_dir(board.path()).unwrap().flatten() {
            found.push(file.path());
        }
    }
    for path in found {
        let ok = wanted
            .iter()
            .any(|w| w.canonicalize().ok() == path.canonicalize().ok());
        assert!(ok, "{} is not written by corpus.rs", path.display());
    }
}

#[test]
fn the_manifest_is_complete_and_every_board_has_a_licence_and_an_origin() {
    let all = boards();
    assert!(all.len() >= 13);
    let mut ids: Vec<&str> = all.iter().map(|b| b.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), all.len(), "board ids are unique");
    for dialect in ["Altium", "Allegro", "EAGLE", "KiCad"] {
        assert!(all.iter().any(|b| b.dialect == dialect), "{dialect}");
    }
    let text = manifest(&all);
    assert_eq!(
        text.matches("\"license\": \"GPL-3.0-or-later\"").count(),
        all.len() + 1
    );
    assert_eq!(text.matches("\"origin\"").count(), all.len() + 1);
    for b in &all {
        assert!(!b.what.is_empty() && !b.files.is_empty(), "{}", b.id);
    }
}

#[test]
fn the_dialect_of_each_board_is_recognised_from_its_files() {
    use eh3d_parser_common::dialect::Dialect;
    for b in boards() {
        let want = match b.dialect {
            "Altium" => Dialect::Altium,
            "Allegro" => Dialect::Allegro,
            "EAGLE" => Dialect::Eagle,
            "KiCad" => Dialect::KiCad,
            _ => Dialect::Generic,
        };
        for f in &b.files {
            let got = match f.kind {
                Kind::Gerber => eh3d_parser_gerber::parse(&f.text).dialect,
                Kind::Drill => eh3d_parser_excellon::parse(&f.text).dialect,
            };
            assert_eq!(got, want, "{} / {}", b.id, f.name);
        }
    }
}

#[test]
fn the_boards_of_the_four_programs_read_without_an_error_and_the_damaged_ones_say_what_is_wrong() {
    use eh3d_parser_gerber::{codes, Severity};
    for b in boards() {
        for f in &b.files {
            let clean = match f.kind {
                Kind::Gerber => eh3d_parser_gerber::parse(&f.text).is_clean(),
                Kind::Drill => eh3d_parser_excellon::parse(&f.text).is_clean(),
            };
            assert_eq!(clean, b.id != "damaged", "{} / {}", b.id, f.name);
        }
    }
    let damaged = &boards()
        .into_iter()
        .find(|b| b.id == "damaged")
        .unwrap()
        .files;
    let g = eh3d_parser_gerber::parse(&damaged[0].text);
    for code in [
        codes::REGION_UNCLOSED,
        codes::UNKNOWN_COMMAND,
        codes::END_MISSING,
    ] {
        assert!(g.diagnostics.iter().any(|d| d.code == code), "{code}");
    }
    let image = eh3d_parser_gerber::image::resolve(&g);
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_CYCLE));
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::ARC_CENTRE_MISSING));
    assert!(g.diagnostics.iter().any(|d| d.severity == Severity::Error));
}

#[test]
fn a_reading_that_is_cut_at_any_point_of_any_file_never_panics() {
    for b in boards() {
        for f in &b.files {
            let chars: Vec<char> = f.text.chars().collect();
            for end in (0..=chars.len()).step_by(7) {
                let prefix: String = chars[..end].iter().collect();
                match f.kind {
                    Kind::Gerber => {
                        let file = eh3d_parser_gerber::parse(&prefix);
                        let _ = eh3d_parser_gerber::snapshot::normalize(&file);
                    }
                    Kind::Drill => {
                        let _ = eh3d_parser_excellon::snapshot::normalize(
                            &eh3d_parser_excellon::parse(&prefix),
                        );
                    }
                }
            }
        }
    }
}
