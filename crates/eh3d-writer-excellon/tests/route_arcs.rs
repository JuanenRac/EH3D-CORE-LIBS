//! The arcs of a route are written as `G02`/`G03` with their radius or offsets and read back as the same route.

use eh3d_parser_excellon::{parse, Point, Route, RouteArc};
use eh3d_writer_excellon::{write, ExcellonOptions, WriteError};

const BASE: &str = "M48\nMETRIC\nT1C2.0\n%\nM30\n";

fn route(arcs: Vec<RouteArc>) -> Route {
    Route {
        line: 1,
        tool: Some(1),
        points: vec![
            Point { x: 0.0, y: 0.0 },
            Point { x: 10.0, y: 0.0 },
            Point { x: 20.0, y: 10.0 },
            Point { x: 10.0, y: 20.0 },
        ],
        arcs,
    }
}

#[test]
fn a_route_with_arcs_by_radius_and_by_offsets_is_written_and_read_back_the_same() {
    let mut file = parse(BASE);
    file.routes.push(route(vec![
        RouteArc {
            to_index: 2,
            clockwise: false,
            radius: Some(10.0),
            center_offset: None,
        },
        RouteArc {
            to_index: 3,
            clockwise: true,
            radius: None,
            center_offset: Some((-10.0, 0.0)),
        },
    ]));
    let text = write(&file, &ExcellonOptions::default()).unwrap();
    assert!(text.contains("G03X20.0Y10.0A10.0\n"), "{text}");
    assert!(text.contains("G02X10.0Y20.0I-10.0J0.0\n"), "{text}");
    assert!(text.contains("G01X10.0Y0.0\n"), "{text}");
    let back = parse(&text);
    assert!(back.is_clean(), "{:?}", back.diagnostics);
    assert_eq!(back.routes.len(), 1);
    assert_eq!(back.routes[0].points, file.routes[0].points);
    assert_eq!(back.routes[0].arcs, file.routes[0].arcs);
    // the same model gives the same bytes
    assert_eq!(text, write(&file, &ExcellonOptions::default()).unwrap());
}

#[test]
fn an_arc_with_neither_radius_nor_offsets_cannot_be_written() {
    let mut file = parse(BASE);
    file.routes.push(route(vec![RouteArc {
        to_index: 1,
        clockwise: true,
        radius: None,
        center_offset: None,
    }]));
    assert_eq!(
        write(&file, &ExcellonOptions::default()),
        Err(WriteError::BadRoute)
    );
}
