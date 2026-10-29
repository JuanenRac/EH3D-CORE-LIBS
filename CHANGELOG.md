# Changelog

All notable changes to this project are documented here.

## [Unreleased]

- **README in the universal layout, in eight languages:** the README has the same sections in the same order as the other repositories of the family (status, overview, architecture, features, security, interfaces, configuration, quick start, testing, structure, roadmap and known limits), with figures taken from the code and from the tests that were run. The Architecture and Repository Structure pictures are drawn from `docs/architecture.json` and `docs/structure.json` by `tools/make_architecture.py` and `tools/make_structure.py`, and the CI checks that they match.
- **The layout of the README is checked in the CI:** the validator runs `tools/universal_readme_check.py` and lists, as a warning, what differs from the universal layout of the README.

## [0.0.1] - Readers for Gerber, Excellon and G-code

- **Gerber (RS-274X) reader** (`eh3d-parser-gerber`): units, the coordinate format with its zero suppression, apertures and macros, regions and the drawing commands. A coordinate wider than its format is refused instead of guessed.
- **Excellon reader** (`eh3d-parser-excellon`): tools, drill hits, slots (`G85`) and simple routes, in millimetres or inches, with leading or trailing zero suppression.
- **G-code reader and modal interpreter** (`eh3d-parser-gcode`): words, comments, line numbers and Marlin checksums; the modal state (units, distance mode, feed, spindle, tool) and every motion in absolute millimetres, arcs included, with the length of the cut and the bounding box. Cutter compensation and canned cycles are reported as unsupported.
- **Diagnostics instead of failures:** each reader returns lines with a severity and the number of the line, and goes on reading; none of them panics on any input.
- 42 tests, `cargo clippy -D warnings` and `cargo fmt --check` in CI on Linux and Windows, and a repository check of the manifest, the versions, the changelog and the eight README files.
