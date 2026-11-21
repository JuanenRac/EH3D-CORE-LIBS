# EH3D-CORE-LIBS: scope

Status: **host-test**. EH3D-CORE-LIBS holds the readers and the writers of the files of the suite: Gerber RS-274X, Excellon drill files and G-code, plus the raster and 2D geometry and the pure CAM that the programs build on. Everything is tried on a computer with test files, damaged files and generated cases; no machine and no collection of real production files has been involved. This page says what it is for, what it will not do and where it sits among the other programs of the suite.

## Purpose

The one place where the files of the suite are read: three readers with no dependencies that never panic and report what they could not understand as diagnostics with line numbers, two writers whose output is the same bytes for the same content, the geometry of a layer, the exact distance on a grid, and the pure CAM (isolation, forced and fill passes, design rules, alignment, mask openings) that draws nothing, reads no file and writes no G-code.

## What it does

* Reads Gerber RS-274X layers (units, coordinate format with its zero suppression, apertures and macros, blocks, regions, step and repeat, transformations and the attributes of X2 and X3) and refuses a coordinate wider than its format instead of guessing.
* Reads Excellon drill files (tools, holes, slots, simple routes with arcs, metric or inches, leading or trailing zeros) and G-code (words, comments, line numbers, Marlin checksums and the modal state of the GRBL, Marlin and PlanetCNC dialects, with every motion in absolute millimetres, arcs included).
* Gives every diagnostic a severity, a stable code and the line of the file, and goes on reading; a file above the size limit is refused and a text that is not UTF-8 is read as Latin-1 with a warning.
* Writes Gerber X2 and Excellon files without dates or versions, so the same content gives the same bytes, and a header and numbers for G-code in one way.
* Turns layers and drill files into shapes in millimetres, computes the booleans of regions with holes, offsets, thick lines, simplification and arc recovery, and the exact Euclidean distance on a grid.
* Plans the pure CAM: isolation passes with the checks for gaps the tool cannot cut, forced and fill passes, design rules and tolerance, ordering and shortening of paths, alignment on fiducials, board transforms and mask openings.

## What it will not do

* No reader of HP-GL or of the Bungard plot files yet, and no support for cutter compensation or canned cycles in G-code: they are reported as unsupported and the block moves nothing.
* No drawing, no file access in the CAM crate and no G-code written by it: the callers pass text in and get data out.
* No decision about whether a program is safe to run on a machine (see below).
* No guessing: a value that does not fit its declared format is refused as a diagnostic, not rounded into something that looks right.

## Where it sits

* **It reads:** text and bytes that the caller passes in: Gerber, Excellon and G-code files, and the limits file `input_limits.cfg` of the shared crate.
* **It writes:** nothing on its own: the writers return the text of a file, and the callers decide where it goes.
* **It is read by:** EH3D-CAM-DEBUGGER, EH3D-STENCILCAM, EH3D-GERBERVIEWER, EH3D-ISOCAM and the other programs of the suite that read these files; EH3D-DRILLPRO still has its own Excellon reader.
* **It depends on:** the Rust standard library only; the crates do not depend on EH3D-COMMON yet.

## What it never decides

The limits of a machine, its interlocks and its emergency stop belong to the machine and to its controller, not to these libraries. A program that was read cleanly is not a verified program: nothing here checks the limits of a machine, the length of a tool or a collision, so a person must check what a program does before anything runs.

The requirements of the program, with the state of each and the test that shows it, are in `REQUIREMENTS.md`; the state in a paragraph is in `STATUS.md`.
