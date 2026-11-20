//! The attributes of X2 and X3 as types.
//!
//! The reader keeps every attribute (`TF`, `TA`, `TO`, `TD`) as a name and a list of values. This module reads
//! the ones the standard defines into enumerations and structures, and keeps what it does not know as
//! [`FileFunction::Other`], [`AperFunction::Other`] and [`ObjectAttribute::Other`], so nothing is lost.
//!
//! None of the readers here fails: a value that does not fit the standard gives the `Other` form, never a panic.

use crate::{Attribute, GerberFile};

/// The side of a layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The top side (`Top`).
    Top,
    /// An inner layer (`Inr`).
    Inner,
    /// The bottom side (`Bot`).
    Bottom,
}

impl Side {
    fn from_text(text: &str) -> Option<Side> {
        match text {
            "Top" => Some(Side::Top),
            "Inr" => Some(Side::Inner),
            "Bot" => Some(Side::Bottom),
            _ => None,
        }
    }
}

/// The kind of a copper layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopperKind {
    /// A signal layer.
    Signal,
    /// A power or ground plane.
    Plane,
    /// A layer with signals and planes.
    Mixed,
    /// A hatched plane.
    Hatched,
}

/// What a drill or a route is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoleKind {
    /// Plated through holes (`PTH`).
    Pth,
    /// Vias (`Blind`, `Buried`).
    Via,
    /// Both (`Mixed`).
    Mixed,
    /// Not given.
    Unspecified,
}

/// `.FileFunction`: what the file is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileFunction {
    /// A copper layer: its number counted from the top, its side and its kind.
    Copper {
        /// The number of the layer, from 1 at the top.
        layer: u32,
        /// The side.
        side: Side,
        /// The kind, when given.
        kind: Option<CopperKind>,
    },
    /// A drill or route file of plated holes, from one copper layer to another.
    Plated {
        /// First layer.
        from: u32,
        /// Last layer.
        to: u32,
        /// What the holes are.
        kind: HoleKind,
    },
    /// A drill or route file of holes that are not plated.
    NonPlated {
        /// First layer.
        from: u32,
        /// Last layer.
        to: u32,
        /// What the holes are.
        kind: HoleKind,
    },
    /// The outline of the board (`Profile,P` plated edge or `Profile,NP`).
    Profile {
        /// True when the edge is plated.
        plated: bool,
    },
    /// A solder mask layer.
    SolderMask {
        /// The side.
        side: Side,
        /// The index when there are several.
        index: Option<u32>,
    },
    /// The silkscreen (`Legend`).
    Legend {
        /// The side.
        side: Side,
        /// The index when there are several.
        index: Option<u32>,
    },
    /// The solder paste.
    Paste {
        /// The side.
        side: Side,
    },
    /// An assembly layer with the components (X3).
    Component {
        /// The number of the layer.
        layer: u32,
        /// The side.
        side: Side,
    },
    /// Something the standard names that is not one of the above, or that does not fit its form.
    Other(String),
}

/// `.FilePolarity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePolarity {
    /// The drawing is the copper (or the material).
    Positive,
    /// The drawing is what is not there.
    Negative,
}

/// `.Part`: what the file describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Part {
    /// One board.
    Single,
    /// An array of boards.
    Array,
    /// The whole panel of fabrication.
    FabricationPanel,
    /// A test coupon.
    Coupon,
    /// Another part, with the text it was given.
    Other(String),
}

/// How a fiducial is meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiducialScope {
    /// Local to a component.
    Local,
    /// For the whole board.
    Global,
    /// For the panel.
    Panel,
}

/// Which opening a pad definition refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadDefinition {
    /// The copper defines the pad (`CuDef`).
    Copper,
    /// The solder mask opening defines the pad (`SMDef`).
    SolderMask,
}

/// `.AperFunction`: what an aperture is for.
#[derive(Debug, Clone, PartialEq)]
pub enum AperFunction {
    /// A drill for a via.
    ViaDrill,
    /// A back drill.
    BackDrill,
    /// A drill for a component, with `true` when it is for a press-fit.
    ComponentDrill {
        /// True for a press-fit.
        press_fit: bool,
    },
    /// A mechanical drill.
    MechanicalDrill,
    /// A castellated drill.
    CastellatedDrill,
    /// Another drill.
    OtherDrill(String),
    /// The pad of a through-hole component.
    ComponentPad,
    /// A surface-mount pad.
    SmdPad(Option<PadDefinition>),
    /// A BGA pad.
    BgaPad(Option<PadDefinition>),
    /// A pad of an edge connector.
    ConnectorPad,
    /// A pad that holds a heatsink.
    HeatsinkPad,
    /// The pad of a via.
    ViaPad,
    /// A test pad.
    TestPad,
    /// A castellated pad.
    CastellatedPad,
    /// A fiducial.
    FiducialPad(FiducialScope),
    /// A thermal relief pad.
    ThermalReliefPad,
    /// A washer pad.
    WasherPad,
    /// An anti-pad.
    AntiPad,
    /// Another pad.
    OtherPad(String),
    /// A track.
    Conductor,
    /// A copper shape that is part of a component.
    EtchedComponent,
    /// Copper that connects nothing.
    NonConductor,
    /// Copper added to balance the layer.
    CopperBalancing,
    /// The border of a panel.
    Border,
    /// Other copper.
    OtherCopper(String),
    /// The body of a component (X3).
    ComponentMain,
    /// The outline of a component (X3).
    ComponentOutline(String),
    /// A pin of a component (X3).
    ComponentPin,
    /// The outline of the board.
    Profile,
    /// A shape that is not material.
    NonMaterial,
    /// A shape of material.
    Material,
    /// Anything else, with the text it was given.
    Other(String),
}

impl AperFunction {
    /// Reads the values of a `.AperFunction` attribute.
    pub fn parse(values: &[String]) -> AperFunction {
        let name = values.first().map(String::as_str).unwrap_or("");
        let second = values.get(1).map(String::as_str);
        let definition = || match second {
            Some("CuDef") => Some(PadDefinition::Copper),
            Some("SMDef") => Some(PadDefinition::SolderMask),
            _ => None,
        };
        match name {
            "ViaDrill" => AperFunction::ViaDrill,
            "BackDrill" => AperFunction::BackDrill,
            "ComponentDrill" => AperFunction::ComponentDrill {
                press_fit: second == Some("PressFit"),
            },
            "MechanicalDrill" => AperFunction::MechanicalDrill,
            "CastelatedDrill" | "CastellatedDrill" => AperFunction::CastellatedDrill,
            "OtherDrill" => AperFunction::OtherDrill(second.unwrap_or("").to_owned()),
            "ComponentPad" => AperFunction::ComponentPad,
            "SMDPad" => AperFunction::SmdPad(definition()),
            "BGAPad" => AperFunction::BgaPad(definition()),
            "ConnectorPad" => AperFunction::ConnectorPad,
            "HeatsinkPad" => AperFunction::HeatsinkPad,
            "ViaPad" => AperFunction::ViaPad,
            "TestPad" => AperFunction::TestPad,
            "CastelatedPad" | "CastellatedPad" => AperFunction::CastellatedPad,
            "FiducialPad" => match second {
                Some("Local") => AperFunction::FiducialPad(FiducialScope::Local),
                Some("Global") => AperFunction::FiducialPad(FiducialScope::Global),
                Some("Panel") => AperFunction::FiducialPad(FiducialScope::Panel),
                _ => AperFunction::Other(values.join(",")),
            },
            "ThermalReliefPad" => AperFunction::ThermalReliefPad,
            "WasherPad" => AperFunction::WasherPad,
            "AntiPad" => AperFunction::AntiPad,
            "OtherPad" => AperFunction::OtherPad(second.unwrap_or("").to_owned()),
            "Conductor" => AperFunction::Conductor,
            "EtchedComponent" => AperFunction::EtchedComponent,
            "NonConductor" => AperFunction::NonConductor,
            "CopperBalancing" => AperFunction::CopperBalancing,
            "Border" => AperFunction::Border,
            "OtherCopper" => AperFunction::OtherCopper(second.unwrap_or("").to_owned()),
            "ComponentMain" => AperFunction::ComponentMain,
            "ComponentOutline" => AperFunction::ComponentOutline(second.unwrap_or("").to_owned()),
            "ComponentPin" => AperFunction::ComponentPin,
            "Profile" => AperFunction::Profile,
            "NonMaterial" => AperFunction::NonMaterial,
            "Material" => AperFunction::Material,
            _ => AperFunction::Other(values.join(",")),
        }
    }
}

/// How a component is mounted (`.CMnt`, X3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountType {
    /// Surface mount.
    Smd,
    /// Through hole.
    ThroughHole,
    /// Press-fit.
    PressFit,
    /// Another way.
    Other,
}

/// An object attribute (`TO`).
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectAttribute {
    /// `.N`: the names of the nets.
    Net(Vec<String>),
    /// `.P`: a pin: the reference of the component, the pin number and, when given, its name.
    Pin {
        /// The reference designator.
        refdes: String,
        /// The pin number.
        number: String,
        /// The name of the pin.
        name: Option<String>,
    },
    /// `.C`: the reference of the component the object belongs to.
    Component(String),
    /// `.CRot`: the rotation of the component in degrees (X3).
    Rotation(f64),
    /// `.CMfr`: the manufacturer (X3).
    Manufacturer(String),
    /// `.CMPN`: the manufacturer part number (X3).
    PartNumber(String),
    /// `.CVal`: the value (X3).
    Value(String),
    /// `.CMnt`: the way it is mounted (X3).
    Mount(MountType),
    /// `.CFtp`: the name of the footprint (X3).
    Footprint(String),
    /// `.CHgt`: the height of the component (X3).
    Height(f64),
    /// Any other attribute, with its name and values.
    Other {
        /// The name, for example `.CPgN`.
        name: String,
        /// The values.
        values: Vec<String>,
    },
}

impl ObjectAttribute {
    /// Reads a `TO` attribute.
    pub fn parse(attribute: &Attribute) -> ObjectAttribute {
        let v = &attribute.values;
        let first = || v.first().cloned().unwrap_or_default();
        let other = || ObjectAttribute::Other {
            name: attribute.name.clone(),
            values: v.clone(),
        };
        match attribute.name.as_str() {
            ".N" => ObjectAttribute::Net(v.clone()),
            ".P" if v.len() >= 2 => ObjectAttribute::Pin {
                refdes: v[0].clone(),
                number: v[1].clone(),
                name: v.get(2).cloned(),
            },
            ".C" if !v.is_empty() => ObjectAttribute::Component(first()),
            ".CRot" => v
                .first()
                .and_then(|t| t.parse().ok())
                .map_or_else(other, ObjectAttribute::Rotation),
            ".CMfr" => ObjectAttribute::Manufacturer(first()),
            ".CMPN" => ObjectAttribute::PartNumber(first()),
            ".CVal" => ObjectAttribute::Value(first()),
            ".CMnt" => ObjectAttribute::Mount(match v.first().map(String::as_str) {
                Some("SMD") => MountType::Smd,
                Some("TH") => MountType::ThroughHole,
                Some("Pressfit") | Some("PressFit") => MountType::PressFit,
                _ => MountType::Other,
            }),
            ".CFtp" => ObjectAttribute::Footprint(first()),
            ".CHgt" => v
                .first()
                .and_then(|t| t.parse().ok())
                .map_or_else(other, ObjectAttribute::Height),
            _ => other(),
        }
    }
}

/// The attributes of the file (`TF`) that the standard defines, read into types.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileAttributes {
    /// `.FileFunction`.
    pub function: Option<FileFunction>,
    /// `.FilePolarity`.
    pub polarity: Option<FilePolarity>,
    /// `.Part`.
    pub part: Option<Part>,
    /// `.GenerationSoftware`: the vendor, the application and, when given, the version.
    pub generation_software: Option<(String, String, Option<String>)>,
    /// `.CreationDate`, as written.
    pub creation_date: Option<String>,
    /// `.ProjectId`: the name, the identifier and the revision.
    pub project_id: Option<(String, String, String)>,
    /// `.SameCoordinates`, with the identifier when there is one.
    pub same_coordinates: Option<String>,
    /// `.MD5`.
    pub md5: Option<String>,
    /// The `TF` attributes that are not in the list above.
    pub other: Vec<Attribute>,
}

fn number(text: Option<&String>) -> Option<u32> {
    let t = text?.strip_prefix('L').unwrap_or(text?);
    t.parse().ok()
}

fn index_after(values: &[String], from: usize) -> Option<u32> {
    values.get(from).and_then(|t| t.parse().ok())
}

impl FileFunction {
    /// Reads the values of a `.FileFunction` attribute.
    pub fn parse(values: &[String]) -> FileFunction {
        let other = || FileFunction::Other(values.join(","));
        let kind = values.first().map(String::as_str).unwrap_or("");
        let side_at = |i: usize| values.get(i).and_then(|t| Side::from_text(t));
        match kind {
            "Copper" => match (number(values.get(1)), side_at(2)) {
                (Some(layer), Some(side)) => FileFunction::Copper {
                    layer,
                    side,
                    kind: match values.get(3).map(String::as_str) {
                        Some("Signal") => Some(CopperKind::Signal),
                        Some("Plane") => Some(CopperKind::Plane),
                        Some("Mixed") => Some(CopperKind::Mixed),
                        Some("Hatched") => Some(CopperKind::Hatched),
                        _ => None,
                    },
                },
                _ => other(),
            },
            "Plated" | "NonPlated" => match (number(values.get(1)), number(values.get(2))) {
                (Some(from), Some(to)) => {
                    let hole = match values.get(3).map(String::as_str) {
                        Some("PTH") => HoleKind::Pth,
                        Some("Blind") | Some("Buried") => HoleKind::Via,
                        Some("Mixed") => HoleKind::Mixed,
                        _ => HoleKind::Unspecified,
                    };
                    if kind == "Plated" {
                        FileFunction::Plated {
                            from,
                            to,
                            kind: hole,
                        }
                    } else {
                        FileFunction::NonPlated {
                            from,
                            to,
                            kind: hole,
                        }
                    }
                }
                _ => other(),
            },
            "Profile" => match values.get(1).map(String::as_str) {
                Some("P") => FileFunction::Profile { plated: true },
                Some("NP") | None => FileFunction::Profile { plated: false },
                _ => other(),
            },
            "Soldermask" => match side_at(1) {
                Some(side) => FileFunction::SolderMask {
                    side,
                    index: index_after(values, 2),
                },
                None => other(),
            },
            "Legend" => match side_at(1) {
                Some(side) => FileFunction::Legend {
                    side,
                    index: index_after(values, 2),
                },
                None => other(),
            },
            "Paste" => match side_at(1) {
                Some(side) => FileFunction::Paste { side },
                None => other(),
            },
            "Component" => match (number(values.get(1)), side_at(2)) {
                (Some(layer), Some(side)) => FileFunction::Component { layer, side },
                _ => other(),
            },
            _ => other(),
        }
    }
}

/// Reads the `TF` attributes of a file into types. An attribute that was deleted by `TD` is not removed:
/// the file attributes live for the whole file.
pub fn file_attributes(file: &GerberFile) -> FileAttributes {
    let mut out = FileAttributes::default();
    for a in file.attributes.iter().filter(|a| a.kind == "TF") {
        let v = &a.values;
        match a.name.as_str() {
            ".FileFunction" => out.function = Some(FileFunction::parse(v)),
            ".FilePolarity" => {
                out.polarity = match v.first().map(String::as_str) {
                    Some("Positive") => Some(FilePolarity::Positive),
                    Some("Negative") => Some(FilePolarity::Negative),
                    _ => None,
                }
            }
            ".Part" => {
                out.part = v.first().map(|t| match t.as_str() {
                    "Single" => Part::Single,
                    "Array" => Part::Array,
                    "FabricationPanel" => Part::FabricationPanel,
                    "Coupon" => Part::Coupon,
                    other => Part::Other(other.to_owned()),
                })
            }
            ".GenerationSoftware" if v.len() >= 2 => {
                out.generation_software = Some((v[0].clone(), v[1].clone(), v.get(2).cloned()))
            }
            ".CreationDate" => out.creation_date = v.first().cloned(),
            ".ProjectId" if v.len() >= 3 => {
                out.project_id = Some((v[0].clone(), v[1].clone(), v[2].clone()))
            }
            ".SameCoordinates" => {
                out.same_coordinates = Some(v.first().cloned().unwrap_or_default())
            }
            ".MD5" => out.md5 = v.first().cloned(),
            _ => out.other.push(a.clone()),
        }
    }
    out
}

/// The `TA.AperFunction` attributes of a file read into types, one for each time one was set, in order.
pub fn aperture_functions(file: &GerberFile) -> Vec<AperFunction> {
    file.attributes
        .iter()
        .filter(|a| a.kind == "TA" && a.name == ".AperFunction")
        .map(|a| AperFunction::parse(&a.values))
        .collect()
}

/// The `TO` attributes of a file read into types, in order.
pub fn object_attributes(file: &GerberFile) -> Vec<ObjectAttribute> {
    file.attributes
        .iter()
        .filter(|a| a.kind == "TO")
        .map(ObjectAttribute::parse)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(text: &str) -> Vec<String> {
        text.split(',').map(str::to_owned).collect()
    }

    #[test]
    fn the_file_functions_of_the_standard_are_read() {
        assert_eq!(
            FileFunction::parse(&values("Copper,L2,Inr,Plane")),
            FileFunction::Copper {
                layer: 2,
                side: Side::Inner,
                kind: Some(CopperKind::Plane)
            }
        );
        assert_eq!(
            FileFunction::parse(&values("Plated,1,4,PTH")),
            FileFunction::Plated {
                from: 1,
                to: 4,
                kind: HoleKind::Pth
            }
        );
        assert_eq!(
            FileFunction::parse(&values("NonPlated,1,2,NPTH")),
            FileFunction::NonPlated {
                from: 1,
                to: 2,
                kind: HoleKind::Unspecified
            }
        );
        assert_eq!(
            FileFunction::parse(&values("Soldermask,Bot,1")),
            FileFunction::SolderMask {
                side: Side::Bottom,
                index: Some(1)
            }
        );
        assert_eq!(
            FileFunction::parse(&values("Legend,Top")),
            FileFunction::Legend {
                side: Side::Top,
                index: None
            }
        );
        assert_eq!(
            FileFunction::parse(&values("Profile,NP")),
            FileFunction::Profile { plated: false }
        );
        assert_eq!(
            FileFunction::parse(&values("Component,L1,Top")),
            FileFunction::Component {
                layer: 1,
                side: Side::Top
            }
        );
        // what does not fit is kept, never a panic
        assert_eq!(
            FileFunction::parse(&values("Copper,Lx,Top")),
            FileFunction::Other("Copper,Lx,Top".into())
        );
        assert_eq!(FileFunction::parse(&[]), FileFunction::Other(String::new()));
    }

    #[test]
    fn the_aperture_functions_of_x2_and_x3_are_read() {
        assert_eq!(
            AperFunction::parse(&values("SMDPad,CuDef")),
            AperFunction::SmdPad(Some(PadDefinition::Copper))
        );
        assert_eq!(
            AperFunction::parse(&values("FiducialPad,Global")),
            AperFunction::FiducialPad(FiducialScope::Global)
        );
        assert_eq!(
            AperFunction::parse(&values("ComponentDrill,PressFit")),
            AperFunction::ComponentDrill { press_fit: true }
        );
        assert_eq!(
            AperFunction::parse(&values("Conductor")),
            AperFunction::Conductor
        );
        assert_eq!(
            AperFunction::parse(&values("ComponentOutline,Courtyard")),
            AperFunction::ComponentOutline("Courtyard".into())
        );
        assert_eq!(
            AperFunction::parse(&values("Strange,1")),
            AperFunction::Other("Strange,1".into())
        );
        assert_eq!(
            AperFunction::parse(&values("FiducialPad,Moon")),
            AperFunction::Other("FiducialPad,Moon".into())
        );
    }

    #[test]
    fn the_object_attributes_of_x2_and_x3_are_read() {
        let a = |name: &str, v: &str| Attribute {
            kind: "TO".into(),
            name: name.into(),
            values: values(v),
        };
        assert_eq!(
            ObjectAttribute::parse(&a(".P", "U1,3,VCC")),
            ObjectAttribute::Pin {
                refdes: "U1".into(),
                number: "3".into(),
                name: Some("VCC".into())
            }
        );
        assert_eq!(
            ObjectAttribute::parse(&a(".N", "GND,AGND")),
            ObjectAttribute::Net(vec!["GND".into(), "AGND".into()])
        );
        assert_eq!(
            ObjectAttribute::parse(&a(".CRot", "90.5")),
            ObjectAttribute::Rotation(90.5)
        );
        assert_eq!(
            ObjectAttribute::parse(&a(".CMnt", "TH")),
            ObjectAttribute::Mount(MountType::ThroughHole)
        );
        assert!(matches!(
            ObjectAttribute::parse(&a(".CRot", "north")),
            ObjectAttribute::Other { .. }
        ));
        assert!(matches!(
            ObjectAttribute::parse(&a(".P", "U1")),
            ObjectAttribute::Other { .. }
        ));
    }

    #[test]
    fn the_file_attributes_of_a_real_header_are_typed() {
        let file = crate::parse(
            "%TF.GenerationSoftware,KiCad,Pcbnew,8.0.1*%\n%TF.CreationDate,2020-01-01T00:00:00+00:00*%\n%TF.ProjectId,demo,00000000-0000-0000-0000-000000000000,rev1*%\n%TF.SameCoordinates,Original*%\n%TF.FileFunction,Copper,L1,Top*%\n%TF.FilePolarity,Positive*%\n%TF.Part,Single*%\n%TF.Extra,1*%\n%FSLAX46Y46*%\n%MOMM*%\nM02*\n",
        );
        let t = file_attributes(&file);
        assert_eq!(
            t.generation_software,
            Some(("KiCad".into(), "Pcbnew".into(), Some("8.0.1".into())))
        );
        assert_eq!(t.polarity, Some(FilePolarity::Positive));
        assert_eq!(t.part, Some(Part::Single));
        assert_eq!(t.same_coordinates.as_deref(), Some("Original"));
        assert_eq!(t.project_id.as_ref().map(|p| p.0.as_str()), Some("demo"));
        assert!(matches!(
            t.function,
            Some(FileFunction::Copper {
                layer: 1,
                side: Side::Top,
                kind: None
            })
        ));
        assert_eq!(t.other.len(), 1);
    }
}
