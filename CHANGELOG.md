# Changelog

All notable changes to this project are documented here.

## [Unreleased]

## [0.0.8] - Coordinate offsets of G92, G28, G30, G53 and G54 to G59, a default format for Excellon files and the grid overflow fix

- **A drill file without a declared format can be read with a format of the caller:** `parse_with_format` and `parse_bytes_with_format` take an `ExcellonDefaults` (`default_format`, `default_zeros`, `default_units`, all optional) that is used only when the file does not declare the same thing, so a program no longer has to write a `FILE_FORMAT` comment into the text and shift its line numbers. What the file declares wins, an empty `ExcellonDefaults` reads as `parse`, and a format out of range is ignored with the new warning `defaults-invalid`. `parse`, `parse_bytes` and `parse_with_units` are unchanged.
- **`G92`, `G28`, `G30`, `G53` and `G54`..`G59` can be followed:** `interpret_coordinates` applies the `G92` offset (and `G92.1`, `G92.2`, `G92.3`) to the positions that follow, sends `G28` and `G30` to reference positions given in `CoordinateOptions` through the optional point of the block, reads `G53` as machine coordinates for its own move and gives each `G54`..`G59` the origin of the configuration; the result exposes the active offsets, and four new diagnostic codes (`reference-position-unset`, `g53-motion-unsupported`, `g92-axes-missing`, `offset-out-of-range`) tell what could not be followed. `interpret` and `interpret_with` are unchanged, and a program that uses none of these commands gives the same trace.
- **The origin the G-code reader assumes is written down:** the position before the first move is X0 Y0 Z0, and the codes that move the origin are told, not followed.
- **A grid that no count can hold no longer overflows:** the memory of a grid just under the largest count is saturated instead of wrapping, and a board of 1e15 mm at the finest cell is refused with a message.

## [0.0.7] - Laser mode, one way to write a number and the header of a G-code file, units given by the caller and a warning for a grid too coarse

- **A hole of an undefined tool is not kept:** the Excellon reader reports the tool and no longer keeps the hole with a diameter of 0.
- **The units of a drill file can be given:** `parse_with_units` reads a file with the units the caller gives (the notices about missing or suspect units go, a file that says other units gets `units-forced`).
- **The laser mode of GRBL can be read:** `interpret_with` with `s0_is_off` reads `M3` or `M4` with `S0` as the laser being off.
- **One way to write a number and a header:** the `emit` module of the G-code crate writes numbers (a value that is not finite is an error, no `-0`), the SHA-256 of a file and the standard header of a G-code file, with the units as a parameter.
- **A grid too coarse for the tool is told:** `resolution_warning` warns when the cell is larger than a quarter of the tool, and the documentation says that the paths follow the copper with a noise of up to half a cell.
- **A scene says that it is incomplete:** `Scene::is_lossy` is true when something of the file was left out.
- **Brand:** Add the application icon and the logos of the program.

## [0.0.6] - Readers accept the largest size limit without overflowing

- **A size limit of the largest possible value no longer overflows:** `read_file_limited` adds one to the limit with a saturating sum, so the caller can pass `usize::MAX` to mean "no limit" without a panic; a test reads a file under that limit.

## [0.0.5] - Macro apertures, arcs by tolerance, bounded readers, strict G-code planes and arcs, and the Gerber and Excellon writers

- **Shared checks updated:** the validator also compares the badge and the Version row of the README with the manifest, fails when one of its checks cannot run, and the CHANGELOG step of the workflow reads the base branch from the environment.
- Add the `eh3d-writer-gerber` crate: a deterministic Gerber X2 writer for the data the Gerber reader gives (apertures, macros, regions, arcs, polarity, step and repeat, file attributes). The same content always gives the same bytes, nothing reads the clock, and a number that does not fit the coordinate format is an error.
- Add the `eh3d-writer-excellon` crate: a deterministic Excellon writer for tools, holes, slots and routes, with coordinates written with a decimal point or in a fixed format with leading or trailing zeros left out.
- Test both writers by writing and reading back with the readers, byte-identical second rounds and generated drawings and drill files.
- **`decode_coordinate("-0")` is plain zero:** a negative zero coordinate no longer comes back as `-0.0`.
- **More tests:** the fiducial fits, the order of the cuts, the lighter paths and the layout of a board (turning, mirroring, panels and alignment holes) in the CAM; the zero suppression, the limits of numbers and the encoding of a file in the shared rules; signs, arcs, polarity and line ends in the Gerber reader; decimals and units in the Excellon reader; comments and line cuts in the G-code reader; the distance transform against a slow exact answer; and the boxes and swept rectangles of the shapes. A star in the comment of a Marlin line is no longer taken for a checksum.
- **Macro apertures are drawn:** the shapes of an aperture macro (circles, vector and center lines, outlines and regular polygons, with variables, arithmetic and rotation) are drawn where the aperture is flashed, so the rounded pads that KiCad writes with a macro are no longer missing; a primitive that clears or has no drawing is still said in the notes.
- **Arcs follow a tolerance:** an arc is cut in as many segments as keep it within 0.005 mm of the true arc (it was always 32), so a large circle is no longer up to 0.1 mm inside; `SceneOptions` and `arc_points_with` set another tolerance.
- **`LR`, `LM` and `LS` are errors when they change the drawing:** a turn, a mirror or a scale of the apertures that was only a warning now gives `geometry-command-unsupported` (the values that change nothing stay a warning), and so does `AB`.
- **The Gerber reader is linear on a file full of draws with no start point:** the last point is kept instead of being searched for in all the commands, and a file keeps at most 100000 diagnostics (`too-many-diagnostics` says the rest were dropped).
- **A file above the limit is refused before it is read:** `read_file_limited` checks the size on disk and reads at most one byte more than the limit.
- **Faster CAM checks:** the pieces that come close are found in one pass over the grid, and a long diagonal track is drawn row by row; a board size out of range is refused instead of wrapping.
- **The direction of a path between two pieces of copper** is decided by all its edges and not by the longest one, and a path with no nearer side is left as it is.
- **Arcs in the wrong plane or with a wrong centre are errors:** after `G18` or `G19` an arc gives `plane-unsupported` as an error and is a line, and an arc given by `I`/`J` whose centre is not as far from its end as from its start gives `arc-radius-mismatch`.
- **Marlin commands with text are not errors:** `M117`, `M118`, `M23`, `M28`, `M29`, `M32`, `M550` and `M551` keep their text as the comment of one line instead of an error for each letter.
- **Every reader is bounded in memory:** the G-code reader keeps at most 5000000 blocks and 100000 diagnostics (`parse_with_limits` sets others), and the Gerber reader reads the text by bytes without copying it into a list of characters.
- **The grid of the CAM tells its memory:** `estimate` gives the cells and the memory (21 bytes a cell) of a layer before the grid is made, the default limit is 30 million cells (it was 60 million) and `new_with_limit` / `from_scene_with_limit` take the limit of the caller.
- **A drill file in inches with no header is noticed:** a file with no units whose holes span less than 5 units gives `units-suspect`, and a hole with a tool that was never defined is said in the notes of the scene.
- **A line with only `%` is not a complaint:** the mark that starts and ends a program in Mach3 and PlanetCNC is read without a warning (a `%` inside a line still is one).
- **The workflows are hardened:** every action is pinned by its commit with its version in a comment, and the checkouts keep no credentials.

## [0.0.4] - Raster, geometry and CAM crates

- **A pure CAM crate:** `eh3d-cam` holds the copper on a grid, the isolation, forced and fill passes, the checks for gaps, tolerance and design rules, the order and the shortening of paths, fiducial alignment, board transforms, solder mask openings and old-style Gerber, so the CAM programs of the suite share one copy; the unit tests of old-style Gerber moved with it and its own tests cover the grid, the helpers and the direction of a path.
- **A geometry crate:** `eh3d-geometry` holds the shapes of a layer (`Scene`, `Shape`, `Bounds`) and the conversion of Gerber and Excellon files into them, so the viewer and the CAM programs share one copy; its own tests cover the bounds, the conversion, the notes, the swept rectangle and the arcs.
- **A raster geometry crate:** `eh3d-raster` holds the units of length (`8mil`, `0.01in`), the exact Euclidean distance transform of a grid and the contour lines of that distance, so the programs of the suite that cut copper share one copy; its own tests cover units, the distance, the contour and the orientation of a path.
- **The CI builds the repositories that depend on this one:** a `consumers` job reads which repositories name this one in `depends_on` of their manifest and runs `cargo check --all-targets` on each with the change in place of the sibling, so a break reaches the author before the merge.

## [0.0.3] - A common crate with stable diagnostic codes, one rule for files without units and a corpus of synthetic files

- **A common crate for the three readers:** `eh3d-parser-common` (internal) holds `Severity`, `Diagnostic`, `Units`, the decoding of coordinates and the parsing of numbers that were written three times; the readers re-export them, so no program that depends on them changes.
- **Stable codes in the diagnostics:** every `Diagnostic` has a `code` (`units-missing`, `number-too-large`, `unknown-g-code`...) so a program can filter by kind instead of by text; the texts are the same and `docs/DIAGNOSTICS.md` lists the codes. A test fails when the table and the code differ. A `Diagnostic` written as a struct literal now needs the `code` field (use `Diagnostic::new`).
- **One rule for files without units:** Gerber and Excellon read them in millimetres and say so with a `units-missing` warning; `effective_units()` gives the unit that was used. The rule and the open decision about inches in old Excellon files are in `docs/DIAGNOSTICS.md`.
- **Numbers have a limit:** a number above 1e15 is refused in the three readers (`number-too-large`), also in G-code.
- **`parse_bytes` in the three readers:** it reads a file from its bytes with a size limit (256 MiB, or your own with `parse_bytes_with`), takes UTF-8 with or without a byte order mark and reads other text as Latin-1 with an `input-not-utf8` warning.
- **A corpus of synthetic files:** `tests/golden/` holds 21 small Gerber, Excellon and G-code files (inches and millimetres, 2.4 and 4.6 formats, `LZ`/`TZ`, slots, routes, arcs, canned cycles, CRLF and LF, Latin-1 text, damaged files), each with the summary every reader must give. `EH3D_REGENERATE=1 cargo test` rewrites the summaries so the change shows in the diff, and every file is also damaged in thousands of ways by the mutation test.
- **No more cuts in the middle of a character:** the three readers no longer slice text by byte index where a non-ASCII character could sit (tool definitions and slots in Excellon, format and aperture commands in Gerber, block-delete lines and long lines in G-code), and `clippy::string_slice` now warns in every crate, so the CI refuses a new one.
- **Mutation tests for the three readers:** a helper without dependencies takes a valid file and builds damaged variants (é, € and an emoji inserted at every position, one character or byte removed, a line repeated, the file cut at every character, extra digits and enormous numbers) and checks that no variant makes a reader panic or loop. 3 new tests, 51 in total.
- **The README says what is true:** the number of tests is the one that runs (51: 16 on Gerber, 13 on Excellon and 22 on G-code) and it names the four programs of the suite that already use the readers.

## [0.0.2] - The README in the universal layout and its check in the CI

- **README in the universal layout, in eight languages:** the README has the same sections in the same order as the other repositories of the family (status, overview, architecture, features, security, interfaces, configuration, quick start, testing, structure, roadmap and known limits), with figures taken from the code and from the tests that were run. The Architecture and Repository Structure pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`, and the CI checks that they match.
- **The layout of the README is checked in the CI:** the validator runs `tools/universal_readme_check.py` and lists, as a warning, what differs from the universal layout of the README.

## [0.0.1] - Readers for Gerber, Excellon and G-code

- **Gerber (RS-274X) reader** (`eh3d-parser-gerber`): units, the coordinate format with its zero suppression, apertures and macros, regions and the drawing commands. A coordinate wider than its format is refused instead of guessed.
- **Excellon reader** (`eh3d-parser-excellon`): tools, drill hits, slots (`G85`) and simple routes, in millimetres or inches, with leading or trailing zero suppression.
- **G-code reader and modal interpreter** (`eh3d-parser-gcode`): words, comments, line numbers and Marlin checksums; the modal state (units, distance mode, feed, spindle, tool) and every motion in absolute millimetres, arcs included, with the length of the cut and the bounding box. Cutter compensation and canned cycles are reported as unsupported.
- **Diagnostics instead of failures:** each reader returns lines with a severity and the number of the line, and goes on reading; none of them panics on any input.
- 42 tests, `cargo clippy -D warnings` and `cargo fmt --check` in CI on Linux and Windows, and a repository check of the manifest, the versions, the changelog and the eight README files.
