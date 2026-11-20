//! Excellon files of the CAD programs, routes with arcs, incremental coordinates, repeats, the attributes of X2
//! written as comments, the notes of Allegro and the limits of the reading.

use eh3d_parser_excellon::{
    codes, parse, parse_with_limits, Dialect, ExcellonLimits, Plating, Severity, Units,
    ZeroSuppression,
};

const HEAD: &str = "M48\nMETRIC\nT1C0.8\nT2C2.0\n%\n";

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn a_route_with_arcs_by_radius_and_by_offsets_keeps_both() {
    let text = format!(
        "{HEAD}T2\nG00X10.0Y10.0\nM15\nG01X20.0Y10.0\nG02X30.0Y20.0A10.0\nG03X20.0Y30.0I-10.0J0.0\nG01X10.0Y30.0\nM16\nM30\n"
    );
    let f = parse(&text);
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    assert_eq!(f.routes.len(), 1);
    let r = &f.routes[0];
    assert_eq!(r.points.len(), 5);
    assert_eq!(r.arcs.len(), 2);
    assert_eq!(r.arcs[0].to_index, 2);
    assert!(r.arcs[0].clockwise);
    assert_eq!(r.arcs[0].radius, Some(10.0));
    assert_eq!(r.arcs[0].center_offset, None);
    assert_eq!(r.arcs[1].to_index, 3);
    assert!(!r.arcs[1].clockwise);
    assert_eq!(r.arcs[1].center_offset, Some((-10.0, 0.0)));
    // the straight pieces are not arcs
    assert!(r.arcs.iter().all(|a| a.to_index != 1 && a.to_index != 4));
}

#[test]
fn an_arc_with_no_radius_and_no_offsets_or_outside_a_route_is_an_error() {
    let no_geometry = parse(&format!(
        "{HEAD}T2\nG00X0Y0\nM15\nG02X10.0Y10.0\nM16\nM30\n"
    ));
    assert!(no_geometry
        .diagnostics
        .iter()
        .any(|d| d.code == codes::ARC_CENTRE_MISSING && d.severity == Severity::Error));
    let outside = parse(&format!("{HEAD}T2\nG02X10.0Y10.0A5.0\nM30\n"));
    assert!(outside
        .diagnostics
        .iter()
        .any(|d| d.code == codes::ARC_UNSUPPORTED));
    assert!(outside.routes.is_empty() && outside.drills.is_empty());
}

#[test]
fn the_mode_of_an_arc_ends_with_the_next_g_code_and_the_end_of_the_route() {
    let f = parse(&format!(
        "{HEAD}T2\nG00X0Y0\nM15\nG02X10.0Y10.0A5.0\nX20.0Y0.0A5.0\nG01X30.0Y0.0\nM16\nG05\nT1\nX1.0Y1.0\nM30\n"
    ));
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    // the second point repeats the arc mode, the third is straight again
    assert_eq!(f.routes[0].arcs.len(), 2);
    assert_eq!(f.drills.len(), 1);
}

#[test]
fn incremental_coordinates_add_to_the_last_position_for_holes_and_slots() {
    let f = parse(&format!(
        "{HEAD}T1\nG91\nX10.0Y10.0\nX5.0\nY-2.0\nG90\nX1.0Y1.0\nG91\nX1.0Y1.0G85X2.0Y0.0\nM30\n"
    ));
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    let at: Vec<(f64, f64)> = f.drills.iter().map(|h| (h.at.x, h.at.y)).collect();
    assert_eq!(
        at,
        vec![(10.0, 10.0), (15.0, 10.0), (15.0, 8.0), (1.0, 1.0)]
    );
    assert_eq!(f.slots.len(), 1);
    assert_eq!((f.slots[0].from.x, f.slots[0].from.y), (2.0, 2.0));
    assert_eq!((f.slots[0].to.x, f.slots[0].to.y), (4.0, 2.0));
    assert!(f
        .diagnostics
        .iter()
        .any(|d| d.code == codes::DEPRECATED_COMMAND));
    let ici = parse("M48\nMETRIC\nICI,ON\nT1C1\n%\nT1\nX1.0Y1.0\nX1.0Y1.0\nM30\n");
    assert!(ici.is_clean());
    assert_eq!(ici.drills[1].at.x, 2.0);
    let off = parse("M48\nMETRIC\nICI,OFF\nT1C1\n%\nT1\nX1.0Y1.0\nX1.0Y1.0\nM30\n");
    assert_eq!(off.drills[1].at.x, 1.0);
    let odd = parse("M48\nMETRIC\nICI,MAYBE\nT1C1\n%\nM30\n");
    assert!(odd
        .diagnostics
        .iter()
        .any(|d| d.code == codes::INCREMENTAL_UNSUPPORTED));
}

#[test]
fn a_repeat_steps_from_the_last_hole_in_the_direction_asked() {
    let f = parse(&format!(
        "{HEAD}T1\nX10.0Y10.0\nR3X-2.0Y1.0\nX0.0Y0.0\nM30\n"
    ));
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    let at: Vec<(f64, f64)> = f.drills.iter().map(|h| (h.at.x, h.at.y)).collect();
    assert_eq!(
        at,
        vec![
            (10.0, 10.0),
            (8.0, 11.0),
            (6.0, 12.0),
            (4.0, 13.0),
            (0.0, 0.0)
        ]
    );
    let before = parse(&format!("{HEAD}T1\nR3X1.0\nM30\n"));
    assert!(before.drills.is_empty());
    assert!(before
        .diagnostics
        .iter()
        .any(|d| d.code == codes::GEOMETRY_COMMAND_UNSUPPORTED));
    let big = parse_with_limits(
        &format!("{HEAD}T1\nX1.0Y1.0\nR200000X1.0\nM30\n"),
        &ExcellonLimits::default(),
    );
    assert_eq!(big.drills.len(), 1, "no hole of a repeat over the limit");
    assert!(big
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
}

#[test]
fn the_attributes_of_x2_in_comments_say_if_the_holes_are_plated() {
    let text = "M48\n; DRILL file {KiCad 8.0.1} date x\n; #@! TF.GenerationSoftware,KiCad,Pcbnew,8.0.1\n; #@! TF.FileFunction,Plated,1,2,PTH\n; #@! TA.AperFunction,Plated,PTH,ViaDrill\nMETRIC\nT1C0.3\n%\nT1\nX1.0Y1.0\nM30\n";
    let f = parse(text);
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    assert_eq!(f.dialect, Dialect::KiCad);
    assert_eq!(f.attributes.len(), 3);
    assert_eq!(f.attributes[1].kind, "TF");
    assert_eq!(f.attributes[1].name, ".FileFunction");
    assert_eq!(f.attributes[1].values, vec!["Plated", "1", "2", "PTH"]);
    assert_eq!(f.plating(), Some(Plating::Plated));
    let non = parse(
        "M48\n; #@! TF.FileFunction,NonPlated,1,2,NPTH\nMETRIC\nT1C3.0\n%\nT1\nX1.0Y1.0\nM30\n",
    );
    assert_eq!(non.plating(), Some(Plating::NonPlated));
    let none = parse("M48\nMETRIC\nT1C3.0\n%\nT1\nX1.0Y1.0\nM30\n");
    assert_eq!(none.plating(), None);
    let altium = parse(
        "M48\n;Layer_Color=9474304\n;TYPE=PLATED\nMETRIC,LZ\nT1F00S00C0.8\n%\nT1\nX1.0Y1.0\nM30\n",
    );
    assert_eq!(altium.dialect, Dialect::Altium);
    assert_eq!(altium.plating(), Some(Plating::Plated));
    assert_eq!(altium.zeros, Some(ZeroSuppression::Trailing));
    let altium_non = parse("M48\n;TYPE=NON_PLATED\nMETRIC\nT1C3.0\n%\nT1\nX1.0Y1.0\nM30\n");
    assert_eq!(altium_non.plating(), Some(Plating::NonPlated));
}

#[test]
fn the_holesize_comments_of_allegro_give_plating_quantity_and_tolerance_of_each_tool() {
    let text = "M48\n; Holesize 1 = 0.600000 Tolerance = +0.100000/-0.050000 PLATED MM Quantity = 12\n; Holesize 2 = 1.000000 Tolerance = +0.000000/-0.000000 PLATED MM Quantity = 3\nMETRIC\nT1C0.6\nT2C1.0\n%\nT1\nX1.0Y1.0\nM30\n";
    let f = parse(text);
    assert_eq!(f.dialect, Dialect::Allegro);
    let n1 = f.notes[&1];
    assert_eq!(n1.plated, Some(true));
    assert_eq!(n1.quantity, Some(12));
    assert!(close(n1.tolerance_plus.unwrap(), 0.1) && close(n1.tolerance_minus.unwrap(), 0.05));
    assert_eq!(f.notes[&2].quantity, Some(3));
    assert_eq!(f.plating(), Some(Plating::Plated));
    let mixed = parse("M48\n; Holesize 1 = 0.6 PLATED MM Quantity = 1\n; Holesize 2 = 1.0 NON_PLATED MM Quantity = 1\nMETRIC\nT1C0.6\nT2C1.0\n%\nM30\n");
    assert_eq!(
        mixed.plating(),
        None,
        "a file with both kinds has no single plating"
    );
    // a comment that is not one, or has no number, is nothing
    let odd = parse("M48\n; Holesize\n; Holesize x = 1\nMETRIC\nT1C1\n%\nM30\n");
    assert!(odd.notes.is_empty());
}

#[test]
fn a_decimal_comma_is_read_as_a_point_with_a_warning() {
    let f = parse("M48\nMETRIC\nT1C0,8\n%\nT1\nX12,5Y3,25\nM30\n");
    assert!(f.is_clean(), "{:?}", f.diagnostics);
    assert!(close(f.tools[&1].diameter, 0.8));
    assert!(close(f.drills[0].at.x, 12.5) && close(f.drills[0].at.y, 3.25));
    let warnings = f
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::DECIMAL_COMMA)
        .count();
    assert_eq!(warnings, 2);
}

#[test]
fn the_formats_of_altium_kicad_and_eagle_headers_are_read_the_same_way() {
    // KiCad: FILE_FORMAT comment, absolute metric with decimals
    let kicad = parse("M48\n; DRILL file {KiCad 8.0.0}\n; FORMAT={-:-/ absolute / metric / decimal}\nFMAT,2\nMETRIC\nT1C0.300\n%\nG90\nG05\nT1\nX100.0Y-50.0\nM30\n");
    assert!(kicad.is_clean(), "{:?}", kicad.diagnostics);
    assert_eq!(
        (kicad.units, kicad.drills[0].at.x, kicad.drills[0].at.y),
        (Some(Units::Millimetres), 100.0, -50.0)
    );
    // Altium: format comment, leading zeros present (LZ), integers
    let altium = parse("M48\n;Layer_Color=9474304\n;FILE_FORMAT=2:5\nINCH,LZ\nT1F00S00C0.0300\n%\nT1\nX010000Y005000\nM30\n");
    assert!(altium.is_clean(), "{:?}", altium.diagnostics);
    assert!(close(altium.drills[0].at.x, 1.0) && close(altium.drills[0].at.y, 0.5));
    // EAGLE: M71 and no format line, 3.3 metric with trailing-zero suppression (TZ)
    let eagle = parse("%\nM48\nM71\nT01C0.8\n%\nT01\nX001500Y002000\nM30\n");
    assert_eq!(eagle.units, Some(Units::Millimetres));
    assert!(close(eagle.drills[0].at.x, 1.5) && close(eagle.drills[0].at.y, 2.0));
}

#[test]
fn the_limits_of_the_reading_stop_it_cleanly() {
    let mut text = String::from(HEAD);
    text.push_str("T1\n");
    for n in 0..100 {
        text.push_str(&format!("X{n}.0Y0.0\n"));
    }
    text.push_str("M30\n");
    let f = parse_with_limits(
        &text,
        &ExcellonLimits {
            max_items: 10,
            ..ExcellonLimits::default()
        },
    );
    assert_eq!(f.drills.len(), 10);
    assert_eq!(
        f.diagnostics
            .iter()
            .filter(|d| d.code == codes::LIMIT_EXCEEDED)
            .count(),
        1
    );
    assert!(!f.ended);
    let tools: String = (1..=50).map(|n| format!("T{n}C1.0\n")).collect();
    let f = parse_with_limits(
        &format!("M48\nMETRIC\n{tools}%\nM30\n"),
        &ExcellonLimits {
            max_tools: 5,
            ..ExcellonLimits::default()
        },
    );
    assert_eq!(f.tools.len(), 5);
    let attrs: String = (0..30).map(|n| format!("; #@! TF.A{n},x\n")).collect();
    let f = parse_with_limits(
        &format!("M48\n{attrs}METRIC\n%\nM30\n"),
        &ExcellonLimits {
            max_attributes: 3,
            ..ExcellonLimits::default()
        },
    );
    assert_eq!(f.attributes.len(), 3);
    assert!(f
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
}
