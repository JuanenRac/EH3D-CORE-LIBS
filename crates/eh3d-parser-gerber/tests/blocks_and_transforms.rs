//! Block apertures (`AB`), step and repeat (`SR`), the transforms of the loaded apertures (`LM`, `LR`, `LS`), the
//! parameters of the whole image and incremental coordinates, read and put in place by `image::resolve`.

use eh3d_parser_gerber::image::{resolve, resolve_with, Affine, ResolveLimits, Segment, Shape};
use eh3d_parser_gerber::{
    codes, parse, parse_with_limits, Aperture, Mirror, Operation, ParseLimits, Polarity, Severity,
};

const HEAD: &str = "%FSLAX24Y24*%\n%MOMM*%\n";

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn bounds_close(got: Option<[f64; 4]>, want: [f64; 4]) -> bool {
    got.is_some_and(|g| g.iter().zip(want).all(|(a, b)| close(*a, b)))
}

const BLOCK: &str = "%ADD10C,0.1*%\n%ADD11R,1X1*%\n%ABD12*%\nD10*\nX0Y0D02*\nX10000Y0D01*\nD11*\nX5000Y5000D03*\n%AB*%\nD12*\nX100000Y100000D03*\nM02*\n";

#[test]
fn a_block_aperture_holds_its_commands_and_is_drawn_where_it_is_flashed() {
    let file = parse(&format!("{HEAD}{BLOCK}"));
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    match file.apertures.get(&12) {
        Some(Aperture::Block { commands }) => assert_eq!(commands.len(), 5),
        other => panic!("{other:?}"),
    }
    // only the selection and the flash of the block are in the main list
    assert_eq!(file.commands.len(), 2);
    let image = resolve(&file);
    assert!(image.diagnostics.is_empty(), "{:?}", image.diagnostics);
    assert_eq!(image.counts(), (1, 1, 0, 0));
    assert_eq!(image.groups.len(), 1);
    assert!(image.items.iter().all(|i| i.group == Some(0)));
    match &image.items[0].shape {
        Shape::Stroke { from, to, .. } => {
            assert_eq!((*from, *to), ((10.0, 10.0), (11.0, 10.0)));
        }
        other => panic!("{other:?}"),
    }
    match &image.items[1].shape {
        Shape::Flash {
            aperture,
            transform,
        } => {
            assert_eq!(*aperture, 11);
            assert_eq!(transform.apply((0.0, 0.0)), (10.5, 10.5));
        }
        other => panic!("{other:?}"),
    }
    // the stroke has a pen of 0.1 and the flash a square of 1
    assert!(
        bounds_close(image.bounds(&file), [9.95, 9.95, 11.05, 11.0]),
        "{:?}",
        image.bounds(&file)
    );
}

#[test]
fn a_block_flashed_twice_makes_two_groups_and_the_state_of_the_file_is_not_changed_by_it() {
    let text = format!("{HEAD}{BLOCK}").replace("M02*\n", "")
        + "X200000Y0D03*\nD10*\nX0Y0D02*\nX10000Y0D01*\nM02*\n";
    let file = parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let image = resolve(&file);
    assert_eq!(image.groups.len(), 2);
    // after the block the aperture and the position of the outer file are the ones it had before
    let last = file.commands.last().unwrap();
    assert!(matches!(last.op, Operation::Line { .. }));
}

#[test]
fn blocks_nest_and_the_inner_polarity_stays_inside_its_group() {
    let text = format!(
        "{HEAD}%ADD10R,1X1*%\n%ABD11*%\nD10*\nX0Y0D03*\n%LPC*%\nX0Y0D03*\n%AB*%\n%ABD12*%\nD11*\nX0Y0D03*\nX20000Y0D03*\n%AB*%\nD12*\nX0Y0D03*\nM02*\n"
    );
    let file = parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let image = resolve(&file);
    assert!(image.diagnostics.is_empty(), "{:?}", image.diagnostics);
    // D12 is one group with two groups of D11 inside it
    assert_eq!(image.groups.len(), 3);
    assert_eq!(image.groups[0].parent, None);
    assert_eq!(image.groups[1].parent, Some(0));
    assert_eq!(image.groups[2].parent, Some(0));
    assert_eq!(image.items.len(), 4);
    let clear = image
        .items
        .iter()
        .filter(|i| i.polarity == Polarity::Clear)
        .count();
    assert_eq!(
        clear, 2,
        "the clear flash of each copy of D11 stays in its group"
    );
    assert!(image.items.iter().all(|i| i.group.is_some()));
    let second = image
        .items
        .iter()
        .filter(|i| i.group == Some(2))
        .map(|i| match &i.shape {
            Shape::Flash { transform, .. } => transform.apply((0.0, 0.0)),
            other => panic!("{other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(second, vec![(2.0, 0.0), (2.0, 0.0)]);
}

#[test]
fn a_block_that_draws_itself_or_another_that_draws_it_is_left_out_with_a_named_error() {
    let direct = parse(&format!(
        "{HEAD}%ADD10C,1*%\n%ABD11*%\nD11*\nX0Y0D03*\n%AB*%\nD11*\nX0Y0D03*\nM02*\n"
    ));
    let image = resolve(&direct);
    assert!(image.items.is_empty());
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_CYCLE && d.severity == Severity::Error));
    // D11 uses D12 and D12 uses D11 (a later definition replaces the first)
    let indirect = parse(&format!(
        "{HEAD}%ABD12*%\nD11*\nX0Y0D03*\n%AB*%\n%ABD11*%\nD12*\nX0Y0D03*\n%AB*%\nD11*\nX0Y0D03*\nM02*\n"
    ));
    let image = resolve(&indirect);
    assert!(image.items.is_empty());
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_CYCLE));
}

#[test]
fn a_chain_of_blocks_deeper_than_the_limit_stops_at_the_limit() {
    let mut text = format!("{HEAD}%ADD10C,1*%\n%ABD11*%\nD10*\nX0Y0D03*\n%AB*%\n");
    for n in 12..=20 {
        text.push_str(&format!("%ABD{n}*%\nD{}*\nX0Y0D03*\n%AB*%\n", n - 1));
    }
    text.push_str("D20*\nX0Y0D03*\nM02*\n");
    let file = parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let deep = resolve(&file);
    assert_eq!(deep.counts().0, 1);
    let shallow = resolve_with(
        &file,
        &ResolveLimits {
            max_depth: 4,
            ..ResolveLimits::default()
        },
    );
    assert_eq!(shallow.counts().0, 0);
    assert!(shallow
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_CYCLE));
}

#[test]
fn a_block_that_is_opened_wrong_or_never_closed_is_an_error_and_the_rest_is_read() {
    let never = parse(&format!(
        "{HEAD}%ADD10C,1*%\n%ABD11*%\nD10*\nX0Y0D03*\nM02*\n"
    ));
    assert!(never
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_INVALID));
    let stray = parse(&format!("{HEAD}%AB*%\nM02*\n"));
    assert!(stray
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_INVALID));
    let small = parse(&format!("{HEAD}%ABD5*%\n%AB*%\nM02*\n"));
    assert!(small
        .diagnostics
        .iter()
        .any(|d| d.code == codes::BLOCK_APERTURE_INVALID));
    let twice = parse(&format!("{HEAD}%ABD11*%\n%AB*%\n%ABD11*%\n%AB*%\nM02*\n"));
    assert!(twice
        .diagnostics
        .iter()
        .any(|d| d.code == codes::APERTURE_REDEFINED));
}

#[test]
fn a_nesting_of_blocks_over_the_limit_of_the_reading_stops_the_reading() {
    let limits = ParseLimits {
        max_block_depth: 2,
        ..ParseLimits::default()
    };
    let file = parse_with_limits(
        &format!("{HEAD}%ABD11*%\n%ABD12*%\n%ABD13*%\n%AB*%\n%AB*%\n%AB*%\nM02*\n"),
        &limits,
    );
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
    assert!(!file.ended);
}

#[test]
fn step_and_repeat_makes_every_copy_in_its_place() {
    let text = format!(
        "{HEAD}%ADD10C,1*%\nD10*\n%SRX3Y2I10J20*%\nX0Y0D03*\nX5000Y0D03*\n%SR*%\nX0Y100000D03*\nM02*\n"
    );
    let file = parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let image = resolve(&file);
    let mut at: Vec<(f64, f64)> = image
        .items
        .iter()
        .map(|i| match &i.shape {
            Shape::Flash { transform, .. } => transform.apply((0.0, 0.0)),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(at.len(), 13, "twelve copies and the flash after the block");
    let last = at.pop().unwrap();
    assert_eq!(last, (0.0, 10.0));
    at.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(at[0], (0.0, 0.0));
    assert_eq!(at[11], (20.5, 20.0));
    assert!(bounds_close(image.bounds(&file), [-0.5, -0.5, 21.0, 20.5]));
}

#[test]
fn a_step_and_repeat_of_a_block_and_of_lines_and_a_region() {
    let text = format!(
        "{HEAD}%ADD10C,0.2*%\nD10*\n%SRX2Y2I5J5*%\nX0Y0D02*\nX10000Y0D01*\nG36*\nX0Y10000D02*\nX10000Y10000D01*\nX10000Y20000D01*\nX0Y10000D01*\nG37*\n%SR*%\nM02*\n"
    );
    let image = resolve(&parse(&text));
    assert_eq!(image.counts(), (0, 4, 0, 4));
}

#[test]
fn a_step_and_repeat_with_too_many_copies_is_refused_and_the_content_is_drawn_once() {
    let file = parse(&format!(
        "{HEAD}%ADD10C,1*%\nD10*\n%SRX100000Y100000I1J1*%\nX0Y0D03*\n%SR*%\nM02*\n"
    ));
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
    assert_eq!(resolve(&file).counts().0, 1);
    // a limit of the resolver too
    let ok = parse(&format!(
        "{HEAD}%ADD10C,1*%\nD10*\n%SRX1000Y1000I1J1*%\nX0Y0D03*\n%SR*%\nM02*\n"
    ));
    assert!(ok.is_clean());
    let capped = resolve_with(
        &ok,
        &ResolveLimits {
            max_items: 1000,
            ..ResolveLimits::default()
        },
    );
    assert_eq!(capped.items.len(), 1000);
    assert!(capped
        .diagnostics
        .iter()
        .any(|d| d.code == codes::IMAGE_TOO_LARGE));
}

#[test]
fn the_transforms_of_the_loaded_apertures_apply_to_flashes_in_the_order_mirror_rotation_scale() {
    let text = format!(
        "{HEAD}%ADD10R,2X1*%\nD10*\n%LR90*%\nX100000Y100000D03*\n%LR0*%\n%LMX*%\n%LR90*%\n%LS2*%\nX200000Y0D03*\nM02*\n"
    );
    let file = parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let image = resolve(&file);
    let t: Vec<Affine> = image
        .items
        .iter()
        .map(|i| match &i.shape {
            Shape::Flash { transform, .. } => *transform,
            other => panic!("{other:?}"),
        })
        .collect();
    // rotated a quarter turn: a point on X goes to Y
    let a = t[0].linear().apply((1.0, 0.0));
    assert!(close(a.0, 0.0) && close(a.1, 1.0), "{a:?}");
    // mirrored first, then turned, then doubled: (1, 0) -> (-1, 0) -> (0, -1) -> (0, -2)
    let b = t[1].linear().apply((1.0, 0.0));
    assert!(close(b.0, 0.0) && close(b.1, -2.0), "{b:?}");
    assert_eq!(t[1].apply((0.0, 0.0)), (20.0, 0.0));
    // the box of the first one: 2 by 1, turned, so 1 by 2
    let only_first = parse(&format!(
        "{HEAD}%ADD10R,2X1*%\nD10*\n%LR90*%\nX100000Y100000D03*\nM02*\n"
    ));
    let b = resolve(&only_first).bounds(&only_first);
    assert!(bounds_close(b, [9.5, 9.0, 10.5, 11.0]), "{b:?}");
}

#[test]
fn a_transform_does_not_turn_a_line_and_applies_to_the_whole_block_when_a_block_is_flashed() {
    let text = format!(
        "{HEAD}%ADD10C,0.2*%\n%ADD11R,1X1*%\n%ABD12*%\nD10*\nX0Y0D02*\nX10000Y0D01*\nD11*\nX20000Y0D03*\n%AB*%\n%LR90*%\nD12*\nX0Y0D03*\nD10*\nX0Y0D02*\nX10000Y0D01*\nM02*\n"
    );
    let file = parse(&text);
    let image = resolve(&file);
    let strokes: Vec<((f64, f64), (f64, f64))> = image
        .items
        .iter()
        .filter_map(|i| match &i.shape {
            Shape::Stroke { from, to, .. } => Some((*from, *to)),
            _ => None,
        })
        .collect();
    // the line of the block was turned with the block, the one drawn in the file was not
    assert_eq!(strokes.len(), 2);
    assert!(
        close(strokes[0].1 .0, 0.0) && close(strokes[0].1 .1, 1.0),
        "{strokes:?}"
    );
    assert_eq!(strokes[1], ((0.0, 0.0), (1.0, 0.0)));
    let flash = image
        .items
        .iter()
        .find_map(|i| match &i.shape {
            Shape::Flash { transform, .. } => Some(transform.apply((0.0, 0.0))),
            _ => None,
        })
        .unwrap();
    assert!(close(flash.0, 0.0) && close(flash.1, 2.0), "{flash:?}");
}

#[test]
fn a_mirror_changes_the_direction_of_an_arc_and_the_drawing_stays_closed() {
    let text = format!(
        "{HEAD}%ADD10C,0.1*%\n%ABD11*%\nD10*\nG75*\nX10000Y0D02*\nG03*\nX0Y10000I-10000J0D01*\n%AB*%\n%LMX*%\nD11*\nX0Y0D03*\nM02*\n"
    );
    let file = parse(&text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    let image = resolve(&file);
    match &image.items[0].shape {
        Shape::ArcStroke {
            from,
            to,
            center,
            clockwise,
            ..
        } => {
            // the quarter circle from (1,0) to (0,1) round (0,0), mirrored about the Y axis: it now goes from
            // (-1,0) to (0,1), clockwise
            assert!(close(from.0, -1.0) && close(from.1, 0.0));
            assert!(close(to.0, 0.0) && close(to.1, 1.0));
            assert!(close(center.0, 0.0) && close(center.1, 0.0));
            assert!(*clockwise);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_parameters_of_the_whole_image_move_the_drawing() {
    let base = format!("{HEAD}%ADD10C,0.2*%\nD10*\nX0Y0D02*\nX20000Y10000D01*\nM02*\n");
    let plain = parse(&base);
    let want = |text: String, box_want: [f64; 4]| {
        let f = parse(&text);
        let b = resolve(&f).bounds(&f);
        assert!(bounds_close(b, box_want), "{text}: {b:?}");
    };
    assert!(bounds_close(
        resolve(&plain).bounds(&plain),
        [-0.1, -0.1, 2.1, 1.1]
    ));
    want(
        format!("{HEAD}%OFA5B-5*%\n%ADD10C,0.2*%\nD10*\nX0Y0D02*\nX20000Y10000D01*\nM02*\n"),
        [4.9, -5.1, 7.1, -3.9],
    );
    want(
        format!("{HEAD}%IR90*%\n%ADD10C,0.2*%\nD10*\nX0Y0D02*\nX20000Y10000D01*\nM02*\n"),
        [-1.1, -0.1, 0.1, 2.1],
    );
    want(
        format!("{HEAD}%MIA1B0*%\n%ADD10C,0.2*%\nD10*\nX0Y0D02*\nX20000Y10000D01*\nM02*\n"),
        [-2.1, -0.1, 0.1, 1.1],
    );
    want(
        format!("{HEAD}%ASAYBX*%\n%ADD10C,0.2*%\nD10*\nX0Y0D02*\nX20000Y10000D01*\nM02*\n"),
        [-0.1, -0.1, 1.1, 2.1],
    );
    want(
        format!("{HEAD}%SFA2B3*%\n%ADD10C,0.2*%\nD10*\nX0Y0D02*\nX20000Y10000D01*\nM02*\n"),
        [-0.2, -0.3, 4.2, 3.3],
    );
    let negative = parse(&format!("{HEAD}%IPNEG*%\n%IN Board 1*%\nM02*\n"));
    assert!(negative.image.negative);
    assert_eq!(negative.image.name.as_deref(), Some("Board1"));
    assert!(resolve(&negative).negative);
}

#[test]
fn incremental_coordinates_add_to_the_last_point() {
    let text = "%FSLIX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX10000Y10000D02*\nX10000Y0D01*\nX0Y10000D01*\nX-5000D01*\nG90*\nX0Y0D02*\nX10000D01*\nM02*\n";
    let file = parse(text);
    assert!(file.is_clean(), "{:?}", file.diagnostics);
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.code == codes::DEPRECATED_COMMAND));
    let ends: Vec<(f64, f64)> = file
        .commands
        .iter()
        .filter_map(|c| match c.op {
            Operation::Move { to } | Operation::Line { to, .. } => Some((to.x, to.y)),
            _ => None,
        })
        .collect();
    assert_eq!(
        ends,
        vec![
            (1.0, 1.0),
            (2.0, 1.0),
            (2.0, 2.0),
            (1.5, 2.0),
            (0.0, 0.0),
            (1.0, 0.0)
        ]
    );
    let g91 = parse(
        "%FSLAX24Y24*%\n%MOMM*%\n%ADD10C,0.1*%\nD10*\nX10000Y10000D02*\nG91*\nX10000D01*\nM02*\n",
    );
    assert!(g91.is_clean());
    assert!(g91
        .commands
        .iter()
        .any(|c| matches!(c.op, Operation::Line { to, .. } if to.x == 2.0)));
}

#[test]
fn regions_with_arcs_keep_their_arcs_in_single_and_in_multi_quadrant_mode() {
    let multi = parse(&format!(
        "{HEAD}G75*\nG36*\nX10000Y0D02*\nG03*\nX0Y10000I-10000J0D01*\nG01*\nX0Y0D01*\nX10000Y0D01*\nG37*\nM02*\n"
    ));
    assert!(multi.is_clean(), "{:?}", multi.diagnostics);
    let single = parse(&format!(
        "{HEAD}G74*\nG36*\nX10000Y0D02*\nG03*\nX0Y10000I10000J0D01*\nG01*\nX0Y0D01*\nX10000Y0D01*\nG37*\nM02*\n"
    ));
    for file in [&multi, &single] {
        let image = resolve(file);
        assert_eq!(image.counts(), (0, 0, 0, 1));
        let Shape::Region { contours } = &image.items[0].shape else {
            panic!()
        };
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].start, (1.0, 0.0));
        match contours[0].segments[0] {
            Segment::Arc {
                to,
                center,
                clockwise,
            } => {
                assert_eq!(to, (0.0, 1.0));
                assert!(close(center.0, 0.0) && close(center.1, 0.0), "{center:?}");
                assert!(!clockwise);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(contours[0].segments.len(), 3);
        assert!(bounds_close(image.bounds(file), [0.0, 0.0, 1.0, 1.0]));
    }
}

#[test]
fn a_region_with_two_contours_gives_two_contours() {
    let file = parse(&format!(
        "{HEAD}G36*\nX0Y0D02*\nG01*\nX10000Y0D01*\nX10000Y10000D01*\nX0Y0D01*\nX30000Y0D02*\nX40000Y0D01*\nX40000Y10000D01*\nX30000Y0D01*\nG37*\nM02*\n"
    ));
    let image = resolve(&file);
    let Shape::Region { contours } = &image.items[0].shape else {
        panic!()
    };
    assert_eq!(contours.len(), 2);
}

#[test]
fn a_pen_of_no_width_is_kept_as_a_hairline_and_a_draw_with_no_aperture_is_said() {
    let file = parse(&format!(
        "{HEAD}%ADD10C,0*%\nD10*\nX0Y0D02*\nX10000Y0D01*\nX10000Y10000D01*\nM02*\n"
    ));
    let image = resolve(&file);
    assert_eq!(image.counts().1, 2);
    assert_eq!(image.hairlines(&file), 2);
    assert!(bounds_close(image.bounds(&file), [0.0, 0.0, 1.0, 1.0]));
    let none = parse(&format!("{HEAD}X0Y0D02*\nX10000Y0D01*\nM02*\n"));
    let image = resolve(&none);
    assert!(image.items.is_empty());
    assert!(image
        .diagnostics
        .iter()
        .any(|d| d.code == codes::APERTURE_UNDEFINED));
}

#[test]
fn the_limits_of_the_reading_stop_it_and_return_what_was_read() {
    let mut text = format!("{HEAD}%ADD10C,1*%\nD10*\n");
    for n in 0..100 {
        text.push_str(&format!("X{n}Y0D03*\n"));
    }
    let limits = ParseLimits {
        max_commands: 10,
        ..ParseLimits::default()
    };
    let file = parse_with_limits(&text, &limits);
    assert_eq!(file.commands.len(), 10);
    assert_eq!(
        file.diagnostics
            .iter()
            .filter(|d| d.code == codes::LIMIT_EXCEEDED)
            .count(),
        1
    );
    let apertures: String = (10..40).map(|n| format!("%ADD{n}C,1*%\n")).collect();
    let file = parse_with_limits(
        &format!("{HEAD}{apertures}M02*\n"),
        &ParseLimits {
            max_apertures: 5,
            ..ParseLimits::default()
        },
    );
    assert_eq!(file.apertures.len(), 5);
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
    let macros: String = (0..20)
        .map(|n| format!("%AMM{n}*\n1,1,1,0,0*%\n"))
        .collect();
    let file = parse_with_limits(
        &format!("{HEAD}{macros}M02*\n"),
        &ParseLimits {
            max_macros: 3,
            ..ParseLimits::default()
        },
    );
    assert_eq!(file.macros.len(), 3);
    let attributes: String = (0..20).map(|n| format!("%TF.A{n},x*%\n")).collect();
    let file = parse_with_limits(
        &format!("{HEAD}{attributes}M02*\n"),
        &ParseLimits {
            max_attributes: 4,
            ..ParseLimits::default()
        },
    );
    assert_eq!(file.attributes.len(), 4);
    assert!(file
        .diagnostics
        .iter()
        .any(|d| d.code == codes::LIMIT_EXCEEDED));
}

#[test]
fn the_new_operations_are_read_in_order_with_their_lines() {
    let file = parse(&format!("{HEAD}%LMXY*%\n%LR45*%\n%LS0.5*%\nM02*\n"));
    let ops: Vec<&Operation> = file.commands.iter().map(|c| &c.op).collect();
    assert_eq!(
        ops,
        vec![
            &Operation::LoadMirror(Mirror::Xy),
            &Operation::LoadRotation(45.0),
            &Operation::LoadScale(0.5)
        ]
    );
    assert_eq!(file.commands[0].line, 3);
}
