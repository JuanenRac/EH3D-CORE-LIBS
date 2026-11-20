# Moving the other Gerber readers of the suite to this one

The suite has three readers of Gerber files: the one in this repository, `phoenix-gerber` (the crate of `PHOENIX-PCB`) and the one of the viewer `EH3D-GERBERVIEWER`. The plan is to keep one, this one, with a set of snapshots as the safety net, so that a change of what a reader understands never goes unseen. The Bungard formats crate of `EH3D-COMMON` stays as it is (see [UNIFICATION.md](UNIFICATION.md)). **This page is the plan; nothing in the other repositories has been changed by it.**

## What the net is

* `tests/corpus/manifest.json` lists fourteen boards (twenty-seven files) written by `crates/eh3d-parser-gerber/tests/corpus.rs`: one board in the dialect of each of Altium, Allegro, EAGLE and KiCad and boards for each feature of the standard. The manifest gives the SHA-256 of each file, its licence and its origin; nothing in the corpus comes from another project.
* Next to each file, `<file>.snap` is the normalised text of its reading: dialect, units, format, attributes, apertures, macros, count of every command, the drawing put in place (flashes, strokes, arcs, regions, groups, hairlines), its box and every diagnostic with line, severity and code. Every length is a whole number of millionths of the unit of the file.
* The test of the corpus rewrites the files, the snapshots and the manifest from the code and fails on any difference.

## The steps, for each repository

1. **Take the corpus.** Copy `tests/corpus/` into the repository (a script that copies it and then checks every SHA-256 against the manifest; never edit a file by hand, because the snapshots belong to the files). Mark it `-text` in `.gitattributes` so Windows does not change the line ends.
2. **Write the adapter that gives the snapshot.** In a test, read each Gerber of the corpus with the reader of the repository and write the same lines as `snapshot::normalize` for the parts the reader has. Start with the lines that decide what is drawn, and list in the test, by name, each line that the reader cannot give yet.
3. **Compare, do not change.** Run the comparison and write down the differences by board and by line. The first run is expected to differ. Each difference is one of: a bug of the old reader (fix it there or leave it to the move), a bug of this reader (open a change here, with a board that shows it added to the corpus), or a choice (write it in the table below).
4. **The tests that must pass before the old reader is replaced**, for each repository:
   * every board of the corpus gives the same `units`, `format`, `apertures` and `drawing` lines through the new path as `kicad`, `altium`, `eagle` and `allegro` snapshots say, with the lines the old reader cannot give listed and agreed;
   * the `bounds` line is the same to the millionth for every board that has no transform of the image (the boards `blocks_and_repeats` and `negative_and_image_parameters` are compared after the transform);
   * `damaged`, `decimal_comma` and `trailing_zeros` give no panic and the same diagnostic codes;
   * the tests of the repository itself, unchanged;
   * for the viewer, the picture of each board of the corpus is compared with the picture the old reader gave (a pixel difference below a stated limit, the limit written in the test).
5. **Put the new reader behind a switch.** A build option or a setting chooses the old or the new reader. Run both in the CI over the corpus and over the repository's own files for a stated number of releases; a difference is a failure.
6. **Remove the old reader** once the new one has been the default for those releases with no difference found, and keep the corpus and the comparison as a permanent test.

## PHOENIX-PCB (`phoenix-gerber`)

* The crate reads Gerber for the layers of a board. Replace its parser by `eh3d_parser_gerber::parse` and its objects by the `Image` of `image::resolve`; the adapter turns `Shape::Flash`, `Stroke`, `ArcStroke` and `Region` into the layer objects of PHOENIX-PCB, and `attributes::file_attributes` gives `.FileFunction` and `.FilePolarity` for the stack-up.
* The dialect and the layer of a file name come from `dialect::detect` and `dialect::layer_from_filename`.
* Pay attention to: the files that PHOENIX-PCB writes (its writer stays; it can use `eh3d-writer-gerber`, which writes `AB`, `LM`, `LR` and `LS` and refuses a negative image instead of dropping it), the netlist attributes (`TO.N`, `TO.P`, `TO.C`) that `attributes::object_attributes` types, and the units (PHOENIX-PCB works in its own unit; convert at the edge).

## EH3D-GERBERVIEWER

* The viewer needs the shapes. Use `image::resolve` for the list and `eh3d-geometry` for the conversion of apertures and macros to shapes (`macros::evaluate` gives every primitive, including the moire and the thermal, with their exposure); clear polarity and groups are composed by the viewer.
* The viewer has the rendering of its own (WebGL2 in the browser, a native one for the desktop); the reader does not change that. The adapter is the function that turns an `Image` into the scene of the renderer.
* Pay attention to: very large files (the limits in `ParseLimits` and `ResolveLimits` are the ones to show the person when a file is cut), the hairline pens (kept and counted by `Image::hairlines`), and the report of what could not be drawn (the diagnostics with their lines, shown with `eh3d_parser_common::i18n::localize` in the language of the person).

## Differences already known

| Point | This reader | Note for the move |
|---|---|---|
| A coordinate with more digits than the format | refused with a diagnostic | The old readers may truncate; the corpus does not contain such a file on purpose. |
| Units of a file that does not say them | millimetres with `units-missing` | Keep the notice visible. |
| Incremental coordinates | followed, with a deprecation warning | Old readers may refuse them. |
| `AB`, `SR`, `LM`, `LR`, `LS` | read and put in place by `image::resolve` | The block is a group; compose it before adding it. |
| Decimal comma | read as a point with `decimal-comma` | |
| Macro primitives | all of the standard evaluated as data | The shapes are the job of the program. |

## What is not covered yet

* Shape conversion of every aperture and macro to polygons in one place (a viewer and a CAM make their own today).
* IPC-D-356A, `.gbrjob` and ODB++ readers.
* A native and a web renderer: they belong to the viewer, not to this repository.

## If something goes wrong

The switch of step 5 is the way back: set the old reader as the default again and keep the comparison running. The corpus and the snapshots stay.
