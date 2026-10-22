# Changelog

All notable changes to this project are documented here.

## [0.0.1] - Readers for Gerber, Excellon and G-code

- **Gerber (RS-274X) reader** (`eh3d-parser-gerber`): units, the coordinate format with its zero suppression, apertures and macros, regions and the drawing commands. A coordinate wider than its format is refused instead of guessed.
- **Excellon reader** (`eh3d-parser-excellon`): tools, drill hits, slots (`G85`) and simple routes, in millimetres or inches, with leading or trailing zero suppression.
- **G-code reader and modal interpreter** (`eh3d-parser-gcode`): words, comments, line numbers and Marlin checksums; the modal state (units, distance mode, feed, spindle, tool) and every motion in absolute millimetres, arcs included, with the length of the cut and the bounding box. Cutter compensation and canned cycles are reported as unsupported.
- **Diagnostics instead of failures:** each reader returns lines with a severity and the number of the line, and goes on reading; none of them panics on any input.
- 42 tests, `cargo clippy -D warnings` and `cargo fmt --check` in CI on Linux and Windows, and a repository check of the manifest, the versions, the changelog and the eight README files.
