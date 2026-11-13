use eh3d_cam::isolate::{
    close_pieces, isolate, label, net_scene, rubout_lines, Params, MAX_RUBOUT_LINES,
};
use eh3d_cam::transform::Axis;
use eh3d_geometry::scene::{Scene, Shape};

fn disc(x: f64, y: f64, r: f64) -> Shape {
    Shape::Disc { x, y, r }
}

fn scene(shapes: Vec<Shape>) -> Scene {
    Scene {
        shapes,
        notes: Vec::new(),
    }
}

#[test]
fn the_parameters_refuse_every_value_out_of_range() {
    assert!(Params::default().validate().is_ok());
    let bad = [
        Params {
            tool_diameter_mm: 0.0,
            ..Params::default()
        },
        Params {
            tool_diameter_mm: f64::NAN,
            ..Params::default()
        },
        Params {
            tool_diameter_mm: 11.0,
            ..Params::default()
        },
        Params {
            stepover_mm: 0.0,
            ..Params::default()
        },
        Params {
            stepover_mm: f64::INFINITY,
            ..Params::default()
        },
        Params {
            res_mm: 0.0,
            ..Params::default()
        },
        Params {
            passes: 0,
            ..Params::default()
        },
        Params {
            passes: 11,
            ..Params::default()
        },
        // a cell wider than half of the tool
        Params {
            res_mm: 0.2,
            ..Params::default()
        },
    ];
    for p in bad {
        assert!(p.validate().is_err(), "{p:?}");
    }
}

#[test]
fn the_reach_grows_with_every_pass_by_one_stepover() {
    let one = Params {
        passes: 1,
        ..Params::default()
    };
    let three = Params {
        passes: 3,
        ..Params::default()
    };
    assert!((one.reach_mm() - 0.1).abs() < 1e-12);
    assert!((three.reach_mm() - 0.3).abs() < 1e-12);
}

#[test]
fn an_isolation_with_invalid_parameters_is_refused_before_any_work() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let p = Params {
        passes: 0,
        ..Params::default()
    };
    assert!(isolate(&s, &p).is_err());
}

#[test]
fn a_disc_is_isolated_with_closed_paths_and_each_pass_goes_farther() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    let p = Params {
        passes: 2,
        ..Params::default()
    };
    let passes = isolate(&s, &p).unwrap();
    assert_eq!(passes.len(), 2);
    assert!(passes[1].offset_mm > passes[0].offset_mm);
    for pass in &passes {
        assert!(!pass.paths.is_empty());
        assert!(pass.cuts().iter().all(|c| c.closed));
    }
}

#[test]
fn the_cuts_of_a_pass_leave_out_degenerate_paths() {
    use eh3d_cam::isolate::Pass;
    let pass = Pass {
        paths: vec![
            vec![(0.0, 0.0), (1.0, 0.0)],
            vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
        ],
        open: vec![vec![(0.0, 0.0)], vec![(0.0, 0.0), (2.0, 0.0)]],
        ..Pass::default()
    };
    let cuts = pass.cuts();
    assert_eq!(cuts.len(), 2);
    assert!(cuts[0].closed && !cuts[1].closed);
}

#[test]
fn the_labels_count_pieces_with_eight_neighbours() {
    // two cells that touch only at a corner are one piece
    let on = [true, false, false, true];
    let (ids, n) = label(&on, 2, 2);
    assert_eq!(n, 1);
    assert_eq!(ids, vec![1, 0, 0, 1]);
    // an empty grid has no pieces
    let (ids, n) = label(&[false; 6], 3, 2);
    assert_eq!(n, 0);
    assert!(ids.iter().all(|i| *i == 0));
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// A union-find over the same grid, to compare with the flood fill of the library.
fn count_with_union_find(on: &[bool], w: usize, h: usize) -> usize {
    let mut parent: Vec<usize> = (0..w * h).collect();
    for y in 0..h {
        for x in 0..w {
            if !on[y * w + x] {
                continue;
            }
            for (dx, dy) in [(1i64, 0i64), (0, 1), (1, 1), (-1, 1)] {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx >= 0
                    && ny >= 0
                    && (nx as usize) < w
                    && (ny as usize) < h
                    && on[ny as usize * w + nx as usize]
                {
                    let a = find(&mut parent, y * w + x);
                    let b = find(&mut parent, ny as usize * w + nx as usize);
                    parent[a] = b;
                }
            }
        }
    }
    let mut roots = std::collections::BTreeSet::new();
    for (i, lit) in on.iter().enumerate() {
        if *lit {
            let r = find(&mut parent, i);
            roots.insert(r);
        }
    }
    roots.len()
}

#[test]
fn the_labels_agree_with_a_union_find_on_random_grids_of_a_fixed_seed() {
    let mut state: u64 = 0x5EED_1234_ABCD_0001;
    let mut next = || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    for _ in 0..300 {
        let (w, h) = (1 + (next() % 12) as usize, 1 + (next() % 12) as usize);
        let density = next() % 100;
        let on: Vec<bool> = (0..w * h).map(|_| next() % 100 < density).collect();
        let (ids, n) = label(&on, w, h);
        assert_eq!(
            n as usize,
            count_with_union_find(&on, w, h),
            "{w}x{h} {on:?}"
        );
        for (i, v) in on.iter().enumerate() {
            assert_eq!(*v, ids[i] != 0);
        }
    }
}

#[test]
fn close_pieces_refuses_a_gap_that_is_not_a_positive_number() {
    let s = scene(vec![disc(0.0, 0.0, 1.0)]);
    for gap in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(close_pieces(&s, gap, 0.05).is_err(), "{gap}");
    }
}

#[test]
fn a_single_piece_and_far_apart_pieces_have_no_close_places() {
    let one = scene(vec![disc(0.0, 0.0, 1.0)]);
    assert!(close_pieces(&one, 0.2, 0.05).unwrap().is_empty());
    let far = scene(vec![disc(0.0, 0.0, 0.5), disc(5.0, 0.0, 0.5)]);
    assert!(close_pieces(&far, 0.2, 0.05).unwrap().is_empty());
}

#[test]
fn net_scene_refuses_a_point_that_is_not_a_number_or_has_no_copper_near() {
    let s = scene(vec![disc(0.0, 0.0, 0.5), disc(5.0, 0.0, 0.5)]);
    assert!(net_scene(&s, (f64::NAN, 0.0), 0.05).is_err());
    assert!(net_scene(&s, (2.5, 0.0), 0.05).is_err());
}

#[test]
fn net_scene_keeps_only_the_shapes_joined_to_the_point() {
    let s = scene(vec![disc(0.0, 0.0, 0.5), disc(5.0, 0.0, 0.5)]);
    let net = net_scene(&s, (0.0, 0.0), 0.05).unwrap();
    assert_eq!(net.shapes.len(), 1);
    let other = net_scene(&s, (5.0, 0.0), 0.05).unwrap();
    assert_eq!(other.shapes.len(), 1);
    assert_ne!(net.shapes, other.shapes);
}

#[test]
fn the_rubout_lines_run_back_and_forth_and_fill_the_area() {
    let pass = rubout_lines((0.0, 0.0, 10.0, 2.0), 1.0, 0.5, Axis::Horizontal).unwrap();
    assert!(pass.rubout);
    assert_eq!(pass.open.len(), 3);
    assert!((pass.open[0][0].0 - 0.5).abs() < 1e-9 && (pass.open[0][1].0 - 9.5).abs() < 1e-9);
    // the second line comes back
    assert!((pass.open[1][0].0 - 9.5).abs() < 1e-9 && (pass.open[1][1].0 - 0.5).abs() < 1e-9);
    // the first and the last line are half a tool from the edges of the area
    assert!((pass.open[0][0].1 - 0.5).abs() < 1e-9);
    assert!((pass.open[2][0].1 - 1.5).abs() < 1e-9);
}

#[test]
fn the_vertical_rubout_lines_swap_the_axes() {
    let pass = rubout_lines((0.0, 0.0, 2.0, 10.0), 1.0, 0.5, Axis::Vertical).unwrap();
    assert_eq!(pass.open.len(), 3);
    assert!((pass.open[0][0].0 - 0.5).abs() < 1e-9 && (pass.open[0][0].1 - 0.5).abs() < 1e-9);
    assert!((pass.open[0][1].1 - 9.5).abs() < 1e-9);
}

#[test]
fn the_rubout_area_may_be_given_with_its_corners_in_any_order() {
    let a = rubout_lines((0.0, 0.0, 10.0, 2.0), 1.0, 0.5, Axis::Horizontal).unwrap();
    let b = rubout_lines((10.0, 2.0, 0.0, 0.0), 1.0, 0.5, Axis::Horizontal).unwrap();
    assert_eq!(a, b);
}

#[test]
fn the_rubout_refuses_bad_tools_overlaps_areas_and_too_many_lines() {
    let area = (0.0, 0.0, 10.0, 2.0);
    assert!(rubout_lines(area, 0.05, 0.5, Axis::Horizontal).is_err());
    assert!(rubout_lines(area, 11.0, 0.5, Axis::Horizontal).is_err());
    assert!(rubout_lines(area, f64::NAN, 0.5, Axis::Horizontal).is_err());
    assert!(rubout_lines(area, 1.0, 0.95, Axis::Horizontal).is_err());
    assert!(rubout_lines(area, 1.0, -0.1, Axis::Horizontal).is_err());
    assert!(rubout_lines((0.0, 0.0, f64::INFINITY, 2.0), 1.0, 0.5, Axis::Horizontal).is_err());
    // not wider than the tool along the lines
    assert!(rubout_lines((0.0, 0.0, 1.0, 5.0), 1.0, 0.5, Axis::Horizontal).is_err());
    // narrower than the tool across the lines
    assert!(rubout_lines((0.0, 0.0, 10.0, 0.5), 1.0, 0.5, Axis::Horizontal).is_err());
    // more lines than the limit
    let tall = (MAX_RUBOUT_LINES as f64) * 2.0;
    assert!(rubout_lines((0.0, 0.0, 10.0, tall), 1.0, 0.0, Axis::Horizontal).is_err());
}

#[test]
fn an_area_exactly_as_wide_as_the_tool_across_makes_one_line() {
    let pass = rubout_lines((0.0, 0.0, 10.0, 1.0), 1.0, 0.5, Axis::Horizontal).unwrap();
    assert_eq!(pass.open.len(), 1);
}
