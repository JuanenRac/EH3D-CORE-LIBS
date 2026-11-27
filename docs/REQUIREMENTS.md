# EH3D-CORE-LIBS: requirements

This register is the single source of the state of each capability: `STATUS.md` and the README files cite it and never state a state that it does not. A state is one of `planned`, `prototype`, `implemented` and `validated`. `validated` means that a person has run the capability on a real machine and the evidence of the manifest says `bench` or `field`; nothing here is validated, because these are libraries that read and write files: they have been tried with test files, damaged files and generated cases on a computer, not on a collection of real production files, and nothing here talks to a machine.

The register has 55 requirements: 48 implemented, 1 prototype, 6 planned and 0 validated. By progress: 48 done, 1 partial, 4 pending and 2 blocked. High coverage is not proof of correctness, so each row says how it is checked: the test that names it, and what it shows.

## How to read the register

* **ID:** `CLIB-NNN`, never reused and never renumbered; a new capability takes the next number.
* **State:** `implemented` is code that is complete and tested on a computer, `prototype` is part of the capability, `planned` is not written.
* **Progress:** `done`, `partial`, `pending` (can be written without hardware) or `blocked` (waits for hardware or for another program of the suite).
* **Evidence:** `simulator` when the test runs against a seeded generator of damaged and random input, `host-test` when it checks a calculation, a reader or a screen-less piece, `none` when nothing is written yet.
* **Test:** the acceptance criterion: `path::name` is the test `name` of `path.rs`, where the path runs from the root of the repository without the extension (for example `crates/eh3d-parser-gcode/tests/gcode::name`, or `crates/eh3d-geometry/src/offset::name` for a test inside a source file). The test must exist. For a capability that is not written the cell says `planned:` and the name of the test that will show it; the day that test exists, the row must change.
* **Limits:** what is not done or not proven; a row that is `partial` or `blocked` always says it.

The check is `crates/eh3d-parser-common/tests/reqs.rs`: it reads this table, finds each test and fails when a state, a test or a count does not agree.

## The Gerber reader

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-001 | A typical copper layer is read completely. | implemented | done | host-test | `crates/eh3d-parser-gerber/tests/gerber::a_typical_copper_layer_is_read_completely` | Test files only; no collection of real production files. |
| CLIB-002 | Omitted coordinates keep their previous value. | implemented | done | host-test | `crates/eh3d-parser-gerber/tests/gerber::omitted_coordinates_keep_their_previous_value` | - |
| CLIB-003 | A quarter circle is read in both directions. | implemented | done | host-test | `crates/eh3d-parser-gerber/src/arc::a_quarter_circle_in_both_directions` | - |
| CLIB-004 | Every primitive of the aperture macros is read, and what cannot be read is listed with its line. | implemented | done | host-test | `crates/eh3d-parser-gerber/src/macros::every_primitive_of_the_standard_is_read` | - |
| CLIB-005 | A block aperture holds its commands and is drawn where it is flashed. | implemented | done | host-test | `crates/eh3d-parser-gerber/tests/blocks_and_transforms::a_block_aperture_holds_its_commands_and_is_drawn_where_it_is_flashed` | - |
| CLIB-006 | The file functions of the standard are read. | implemented | done | host-test | `crates/eh3d-parser-gerber/src/attributes::the_file_functions_of_the_standard_are_read` | - |
| CLIB-007 | A coordinate is read back without loss in every format. | implemented | done | host-test | `crates/eh3d-parser-gerber/tests/properties::a_coordinate_is_read_back_without_loss_in_every_format` | Seeded generation, not a continuous fuzzer. |
| CLIB-008 | The corpus of boards has a manifest with a licence and an origin for every board, and the readings are the stored snapshots. | implemented | done | host-test | `crates/eh3d-parser-gerber/tests/corpus::the_manifest_is_complete_and_every_board_has_a_licence_and_an_origin` | The boards are written by a test in the dialects of the design tools; they are not files from those tools. |

## The Excellon reader

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-009 | A typical metric file is read. | implemented | done | host-test | `crates/eh3d-parser-excellon/tests/excellon::a_typical_metric_file_is_read` | Test files only. |
| CLIB-010 | Slots with G85 are read. | implemented | done | host-test | `crates/eh3d-parser-excellon/tests/excellon::slots_with_g85_are_read` | - |
| CLIB-011 | What the file declares wins over the defaults. | implemented | done | host-test | `crates/eh3d-parser-excellon/tests/defaults::what_the_file_declares_wins_over_the_defaults` | - |
| CLIB-012 | A route with arcs by radius and by offsets keeps both. | implemented | done | host-test | `crates/eh3d-parser-excellon/tests/dialects_and_routes::a_route_with_arcs_by_radius_and_by_offsets_keeps_both` | - |
| CLIB-013 | A decimal comma gives the same holes as a decimal point. | implemented | done | host-test | `crates/eh3d-parser-excellon/tests/field_errors::a_decimal_comma_gives_the_same_holes_as_a_decimal_point` | - |
| CLIB-014 | The two Excellon readers find the same holes for each tool in the plain drill files. | implemented | done | host-test | `crates/eh3d-parser-excellon/tests/differential::both_readers_find_the_same_holes_for_each_tool_in_the_plain_drill_files` | Compares with the reader of EH3D-DRILLPRO as it stands. |

## The G-code reader and interpreter

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-015 | A small milling program is followed. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/gcode::a_small_milling_program_is_followed` | - |
| CLIB-016 | Inches become millimetres, including the feed. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/gcode::inches_become_millimetres_including_the_feed` | - |
| CLIB-017 | Relative mode adds to the position. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/gcode::relative_mode_adds_to_the_position` | - |
| CLIB-018 | G92 shifts the positions that follow. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/coordinates::g92_shifts_the_positions_that_follow` | - |
| CLIB-019 | A checksum that is not a number is an error and the words are still read. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/more::a_checksum_that_is_not_a_number_is_an_error_and_the_words_are_still_read` | - |
| CLIB-020 | The modal state is a value and nothing is kept between programs. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/field_errors::the_modal_state_is_a_value_and_nothing_is_kept_between_programs` | - |
| CLIB-021 | What the G-code writer writes is read back by the reader of the same crate. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/emit::what_is_written_is_read_back_by_the_reader_of_the_same_crate` | - |
| CLIB-022 | The lines of the shared table give the words the server gives. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/shared_words::the_lines_of_the_shared_table_give_the_words_the_server_gives` | - |

## Input limits, diagnostics and damaged files

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-023 | A Gerber file above the size limit is refused. | implemented | done | host-test | `crates/eh3d-parser-gerber/tests/input::a_file_above_the_limit_is_refused` | - |
| CLIB-024 | Text that is not UTF-8 is read as Latin-1 with a warning. | implemented | done | host-test | `crates/eh3d-parser-excellon/tests/input::latin1_text_is_read_with_a_warning` | - |
| CLIB-025 | A hostile file ends with a named reason and what was read. | implemented | done | host-test | `crates/eh3d-parser-gerber/tests/field_errors::a_hostile_file_ends_with_a_named_reason_and_what_was_read` | - |
| CLIB-026 | No variant of a golden file breaks the reader. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/golden::no_variant_of_a_golden_file_breaks_the_reader` | Thousands of damaged variants from a fixed seed, not a continuous fuzzer. |
| CLIB-027 | A number is accepted up to the magnitude limit and not a hair beyond. | implemented | done | host-test | `crates/eh3d-parser-common/tests/common::a_number_is_accepted_up_to_the_magnitude_limit_and_not_a_hair_beyond` | - |
| CLIB-028 | The headlines of the diagnostic codes are in the eight languages, and the Portuguese is that of Portugal. | implemented | done | host-test | `crates/eh3d-parser-common/src/i18n::portuguese_is_the_one_of_portugal_and_not_spanish_or_brazilian` | - |
| CLIB-029 | The limits file is valid and a bad one is refused with its line. | implemented | done | host-test | `crates/eh3d-parser-common/src/limits::a_bad_configuration_is_refused_with_its_line` | - |

## The writers

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-030 | Gerber written and read back twice gives the same bytes the second time. | implemented | done | host-test | `crates/eh3d-writer-gerber/src/lib::the_second_round_is_byte_identical` | Read back with the reader of this repository only; no viewer has opened the files. |
| CLIB-031 | A block that needs an aperture that is not there, or a cycle, is refused. | implemented | done | host-test | `crates/eh3d-writer-gerber/tests/blocks_and_transforms::a_block_that_needs_an_aperture_that_is_not_there_or_a_cycle_is_refused` | - |
| CLIB-032 | A negative image or the deprecated parameters are refused instead of dropped. | implemented | done | host-test | `crates/eh3d-writer-gerber/tests/blocks_and_transforms::a_negative_image_or_the_deprecated_parameters_are_refused_instead_of_dropped` | - |
| CLIB-033 | Excellon written and read back gives the same model in every style. | prototype | partial | host-test | `crates/eh3d-writer-excellon/src/lib::write_then_read_gives_the_same_model_in_every_style` | No drill program or machine has opened the files. |
| CLIB-034 | The same seed gives the same Excellon file and the file reads clean. | implemented | done | host-test | `crates/eh3d-writer-excellon/tests/simulation::the_same_seed_gives_the_same_file_and_the_file_reads_clean` | - |
| CLIB-035 | An arc with neither radius nor offsets cannot be written. | implemented | done | host-test | `crates/eh3d-writer-excellon/tests/route_arcs::an_arc_with_neither_radius_nor_offsets_cannot_be_written` | - |

## Geometry and raster

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-036 | The distance on a grid matches the slow exact answer on grids of every shape. | implemented | done | host-test | `crates/eh3d-raster/tests/more::the_distance_matches_the_slow_exact_answer_on_grids_of_every_shape` | - |
| CLIB-037 | The union of random rectangles has the area counted cell by cell. | implemented | done | host-test | `crates/eh3d-geometry/tests/geometry2d::the_union_of_random_rectangles_has_the_area_counted_cell_by_cell` | - |
| CLIB-038 | The four operations agree with each other on random polygons. | implemented | done | host-test | `crates/eh3d-geometry/tests/geometry2d::the_four_operations_agree_with_each_other_on_random_polygons` | Seeded random shapes. |
| CLIB-039 | A Gerber layer and a drill file become shapes in millimetres. | implemented | done | host-test | `crates/eh3d-geometry/tests/scene::a_gerber_layer_and_a_drill_file_become_shapes_in_millimetres` | - |
| CLIB-040 | A square grown is a square with rounded corners. | implemented | done | host-test | `crates/eh3d-geometry/src/offset::a_square_grown_is_a_square_with_rounded_corners` | - |
| CLIB-041 | A quarter circle of pieces becomes one arc with its centre and radius. | implemented | done | host-test | `crates/eh3d-geometry/src/arcfit::a_quarter_circle_of_pieces_becomes_one_arc_with_its_centre_and_radius` | - |
| CLIB-042 | Degenerate rings have no area and no centroid, and do not panic. | implemented | done | host-test | `crates/eh3d-geometry/src/polygon::degenerate_rings_have_no_area_no_centroid_and_do_not_panic` | - |

## The pure CAM

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-043 | The parameters of an isolation refuse every value out of range, before any work. | implemented | done | host-test | `crates/eh3d-cam/tests/isolate::the_parameters_refuse_every_value_out_of_range` | - |
| CLIB-044 | Two tracks closer than the rule are a clearance violation near the gap. | implemented | done | host-test | `crates/eh3d-cam/tests/drc::two_tracks_closer_than_the_rule_are_a_clearance_violation_near_the_gap` | - |
| CLIB-045 | A square is filled with lines one tool apart that go back and forth. | implemented | done | host-test | `crates/eh3d-cam/tests/fill::a_square_is_filled_with_lines_one_tool_apart_that_go_back_and_forth` | - |
| CLIB-046 | A gap narrower than the tool but not than the minimum gets a forced path. | implemented | done | host-test | `crates/eh3d-cam/tests/forced::a_gap_narrower_than_the_tool_but_not_than_the_minimum_gets_a_path_where_the_copper_is_near` | - |
| CLIB-047 | Three marks give the exact affine transformation and four a least-squares fit. | implemented | done | host-test | `crates/eh3d-cam/tests/geometry_ops::three_marks_give_the_exact_affine_transformation_and_four_marks_a_least_squares_fit` | - |
| CLIB-048 | Without a mask layer the pads are opened grown by the margin. | implemented | done | host-test | `crates/eh3d-cam/tests/mask::without_a_mask_layer_the_pads_are_opened_grown_by_the_margin_and_the_tracks_stay_covered` | - |
| CLIB-049 | Rules outside their range are refused with the name of the rule. | implemented | done | host-test | `crates/eh3d-cam/tests/problems::rules_outside_their_range_are_refused_with_the_name_of_the_rule` | - |

## Not written yet

| ID | Requirement | State | Progress | Evidence | Test | Limits |
|---|---|---|---|---|---|---|
| CLIB-050 | A reader of HP-GL and Bungard plot files. | planned | pending | none | `planned:crates/eh3d-parser-hpgl/tests/hpgl::a_plot_file_is_read_with_its_pens_and_paths` | Not written; the programs that need it have their own readers. |
| CLIB-051 | Cutter compensation and canned cycles are read by the G-code interpreter. | planned | pending | none | `planned:crates/eh3d-parser-gcode/tests/canned::a_canned_cycle_is_followed` | Reported as unsupported today; the block moves nothing. |
| CLIB-052 | The other Gerber and Excellon readers of the suite are moved to these readers. | planned | pending | none | `planned:crates/eh3d-parser-gerber/tests/migration::the_corpus_passes_on_the_reader_that_is_replaced` | A plan exists; both pairs stay and two differential tests keep them in agreement. |
| CLIB-053 | The readers are tried on a collection of real production files from several design tools. | planned | blocked | none | `planned:crates/eh3d-parser-gerber/tests/real_files::real_production_files_are_read_clean` | Needs real files and their licences. |
| CLIB-054 | A viewer or a machine program opens the files that the writers make. | planned | blocked | none | `planned:crates/eh3d-writer-gerber/tests/viewer::a_viewer_opens_the_files_of_the_writer` | Needs another program to open them. |
| CLIB-055 | The crates use the types of EH3D-COMMON. | planned | pending | none | `planned:crates/eh3d-parser-common/tests/common_types::the_diagnostics_use_the_types_of_the_common_library` | Not written; the crates stand on their own today. |
| CLIB-056 | What a controller would run differently from what was read is an error: split numbers, commands of the controller, what is left out of a line, blocks after the end. | implemented | done | host-test | `crates/eh3d-parser-gcode/tests/refusal::spaces_and_comments_inside_a_number_count_for_nothing_as_for_grbl` | Read by the rules of the protocol; not compared with a controller. |
