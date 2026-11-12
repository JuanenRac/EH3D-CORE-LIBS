# One reader for Gerber and one for Excellon: a plan

The suite reads the same two kinds of file with two different pairs of readers:

* **These crates** (`eh3d-parser-gerber`, `eh3d-parser-excellon`): strict about the formats, with a diagnostic and a line number for everything they do not understand. Used by `EH3D-GERBERVIEWER`, `EH3D-ISOCAM`, `EH3D-STENCILCAM` and `EH3D-CAM-DEBUGGER`.
* **The Bungard formats crate** of `EH3D-COMMON` (`eh3d-bungard-formats`: `gerber.rs`, `ncd.rs`, `hpgl.rs`): written for the files of the Bungard machines, which count in mils or in plotter steps, and used by `EH3D-DRILLPRO`, `EH3D-ROUTEPRO`, `EH3D-BUNGARD-GUI` and (for the drill and outline files) `EH3D-ISOCAM`.

The same file can therefore be drawn correctly by the viewer and be read differently by the program that drives the machine. This page is the plan to end that; it is not done yet, and nothing here changes a reader.

The crates added since this page was written (`eh3d-geometry`, `eh3d-cam`, `eh3d-raster` and the Gerber and Excellon writers) build on the readers of this repository and do not change them, and `EH3D-COMMON` still does not depend on this repository, so the two pairs of readers are still two.

## Where they differ today

| Point | These crates | Bungard formats crate |
|---|---|---|
| A coordinate with more digits than the declared format | refused with a diagnostic | taken as it comes |
| Zero suppression (`LZ`/`TZ`) and `METRIC`/`INCH` of the drill file | applied to the coordinates | read, but not applied to the coordinates in every case |
| `G01`/`G02`/`G03` written without a zero, and the modal `D` code | read | read since the last fix; `G74` is not recognised |
| Diagnostics | line, severity and message for each | few, without line numbers |
| Slots (`G85`), routes (`M15`/`G01`/`M16`) | read | slots are not held by the hole format |
| Units of the result | millimetres | mils, tenths of a mil, plotter steps or hundredths of a millimetre, as the machine file says |

## The plan, in steps that each leave everything working

1. **A corpus both readers must agree on.** Put a set of small Gerber and Excellon files (typical ones, files with omitted zeros, with slots, with arcs, and damaged ones) in one place and a test that reads each with both readers and compares the points they give in millimetres. The first run lists the real differences; nothing is changed until then.
2. **An adapter in the formats crate, not a rewrite.** The formats crate keeps what is only Bungard's (the unit of the machine file, the files in mils, the DOS names and limits, the plotter steps) and gets its Gerber and Excellon input from these crates: it asks them for millimetres and converts to the unit of the machine at the edge. The public functions of the formats crate keep their signatures, so the programs that use it do not change.
3. **Gerber first, then Excellon.** Gerber is the smaller change (the formats crate only needs the flashes, the lines and the arcs). Excellon follows, with the slots made explicit: a program that cannot hold a slot must say so instead of dropping it.
4. **The diagnostics reach the person.** A line that was refused or guessed is shown by the programs that drive a machine (today they say nothing), because a hole read wrongly is a hole drilled in the wrong place.
5. **Remove the old readers** once the corpus passes with the adapter and the programs' own tests pass; keep the corpus as a test of both crates.

## What to watch for

* The Bungard files are not a variant of the standard ones: a coordinate there may be whole mils with no point and no format line. The adapter has to take the unit and the digits from the machine file, never guess them.
* The result of drilling must not change for a file that is already read correctly: the corpus includes the files the programs use as examples, and their holes are compared before and after.
* These crates stay free of dependencies: the dependency goes from the formats crate to them, never the other way round.

## Decision: both readers stay

The plan above is **not carried out**. The owner of the suite decided to keep both pairs of readers, the ones of these crates and the ones of the Bungard formats crate of `EH3D-COMMON`, and to make sure they agree where they must with a test instead of joining them: no dependency from the formats crate to these crates and no new crate. The comparison of step 1 exists and runs in the CI of this repository: `crates/eh3d-parser-excellon/tests/differential.rs` (the same number of holes with the same tools in the same order, and how far apart the positions are) and `crates/eh3d-parser-gerber/tests/differential.rs` (every end of the outline lies inside the box of the path the other reader makes). The two crates of the formats are therefore development dependencies of the tests here (the CI checks `EH3D-COMMON` out next to this repository). A new difference between the readers is found by those tests, not by a person.
