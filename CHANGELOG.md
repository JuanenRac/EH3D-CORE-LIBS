# Changelog

All notable changes to this project are documented here.

## [Unreleased]
- **`decode_coordinate("-0")` is plain zero:** a negative zero coordinate no longer comes back as `-0.0`.
- **More tests:** the fiducial fits, the order of the cuts, the lighter paths and the layout of a board (turning, mirroring, panels and alignment holes) in the CAM; the zero suppression, the limits of numbers and the encoding of a file in the shared rules; signs, arcs, polarity and line ends in the Gerber reader; decimals and units in the Excellon reader; comments and line cuts in the G-code reader; the distance transform against a slow exact answer; and the boxes and swept rectangles of the shapes. A star in the comment of a Marlin line is no longer taken for a checksum.

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
