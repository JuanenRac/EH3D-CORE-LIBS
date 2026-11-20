# What each reader promises

All three readers (`eh3d-parser-gerber`, `eh3d-parser-excellon`, `eh3d-parser-gcode`) share the same rules.

## The rules

* **No panic.** Any text, however damaged, comes back as a result. The tests feed them empty files, binary garbage, huge numbers, unterminated comments and lines of thousands of characters.
* **No guess.** A value that does not fit its declared format is refused (`diagnostic`), not rounded or truncated into something that looks right.
* **Diagnostics.** Each problem is a `Diagnostic { line, severity, code, message }`: `Info` (worth knowing), `Warning` (probably still right) or `Error` (something was lost). `is_clean()` is true when there is no `Error`. The `code` is a stable identifier of the kind of notice (see [DIAGNOSTICS.md](DIAGNOSTICS.md)): filter on it, not on the text.
* **Shared code.** `Severity`, `Diagnostic`, `Units`, the decoding of coordinates and the limits on numbers live in the internal crate `eh3d-parser-common`; the readers re-export them, so a program only depends on the reader it uses.
* **Files.** `read_file_limited` refuses a file above the limit by its size on disk, before it is read into memory.
* **Bytes.** `parse_bytes` reads a file from its bytes with a size limit (256 MiB by default) and reads text that is not UTF-8 as Latin-1, with an `input-not-utf8` warning.
* **Units.** A Gerber or Excellon file that does not declare its units is read in millimetres, with a `units-missing` warning.
* **Bounded.** The G-code reader cuts a line at 512 characters and says so. A number above 1e15 is refused with `number-too-large`. A program keeps at most 5000000 blocks (`program-too-long`) and every reader at most 100000 diagnostics (`too-many-diagnostics`); `parse_with_limits` sets other limits.

## Gerber (RS-274X)

Units, the coordinate format (integer and decimal digits, leading or trailing zero suppression, absolute or incremental coordinates), apertures (circle, rectangle, obround, polygon, macro and block), regions with arcs in single and multi quadrant mode, step and repeat, the polarity, the mirror, turn and scale of the loaded apertures (`LM`, `LR`, `LS`), the deprecated parameters of the whole image (`IP`, `OF`, `SF`, `MI`, `IR`, `AS`, `IN`), the attributes of X2 and X3 (also those that KiCad writes as `G04 #@!` comments) and the drawing commands. A coordinate with more digits than its format allows is an error. A number written with a decimal comma is read as a point with a warning. The reader keeps at most 100000 diagnostics per file and stops at the limits of `ParseLimits` (commands, apertures, macros, attributes, nesting of blocks, copies of a repeat).

`image::resolve` puts the file in place (blocks expanded, repeats made, transforms applied, arcs worked out, regions as contours), `macros::evaluate` evaluates a macro (the primitives 1, 2, 4, 5, 6, 7, 20, 21 and 22, variables and arithmetic), `attributes` types the attributes, `dialect::detect` recognises Altium, Allegro, EAGLE and KiCad, and `snapshot::normalize` writes the text the corpus compares. The list of what a program can rely on is in [API.md](API.md).

## Excellon

Tools with diameter, feed and speed; drill hits; slots with `G85`; routes (`M15`, `G00`, `G01`, `M16`) with arcs (`G02`/`G03` with the radius `A` or the offsets `I`/`J`); the repeat `R<n>X<step>Y<step>`; incremental coordinates (`G91`, `ICI,ON`); metric or inch; `TZ` (leading zeros suppressed) or `LZ` (trailing zeros suppressed); coordinates with a decimal point are read as they are, and so are those with a decimal comma (with a warning). `FILE_FORMAT=i:d` comments are honoured, the attributes of X2 written as `; #@! TF...` comments are kept, the `Holesize` comments of Allegro and `TYPE=PLATED` of Altium give the plating (`ExcellonFile::plating()`), and `ExcellonLimits` bounds the work.

A file that does not declare its format can be read with the assumptions of the caller: `parse_with_format(text, &ExcellonDefaults { default_format: Some((2, 4)), default_zeros: Some(ZeroSuppression::Leading), default_units: Some(Units::Inches) })` (`parse_bytes_with_format` takes bytes). Every field is optional and `ExcellonDefaults::default()` reads exactly as `parse`. What the file declares always wins (the `FILE_FORMAT` comment, `INCH,TZ,00.0000`, `METRIC`, `M71`, `M72`), the lines of the diagnostics are those of the text, and when the units come from the defaults there is no `units-missing` or `units-suspect` notice. A default format outside 1 to 8 integer digits and 0 to 8 decimal digits is ignored with the warning `defaults-invalid` at line 1 (`ExcellonDefaults::is_valid` tells it beforehand).

## G-code

Dialects: GRBL, Marlin (line numbers and checksums, verified; `M117`, `M118`, `M23` and the like with text are kept as one line) and PlanetCNC. `interpret` walks the program the way a controller does: `G20`/`G21`, `G90`/`G91`, `G90.1`/`G91.1`, `G0`..`G3` (arcs with `I`/`J` or `R`), `F`, `S`, `T`, `M3`/`M4`/`M5`, `M2`/`M30`. Every motion comes out in absolute millimetres, with the feed in millimetres per minute.

## Coordinate offsets (opt in)

`interpret_coordinates(&program, &InterpretOptions, &CoordinateOptions)` walks a program like `interpret_with` and also follows the commands that move the origin. A program that uses none of them gives the same trace as `interpret_with`; `interpret` and `interpret_with` themselves are unchanged. The positions of the trace are machine coordinates in millimetres, and the machine starts at X0 Y0 Z0.

* `G92` makes the work position of the axes written the value given, without moving (the offset is recomputed, not added up); `G92.1` cancels it, `G92.2` suspends it and `G92.3` applies it again. The result has `state.offset` (what is added to a work coordinate), `state.g92_offset` and `state.g92_active`.
* `G54`..`G59` select a work coordinate system whose origin is `CoordinateOptions::work_offsets` (all zero by default); `state.work_system` and `state.work_offset` say which one is active.
* `G53` makes the move of its own block (`G0` or `G1`) in machine coordinates, absolute, with no offset; with an arc it is the error `g53-motion-unsupported` and the block moves nothing.
* `G28` and `G30` go through the point of the block (not in Marlin, where the axes only name what goes home) and then to `reference_g28` / `reference_g30` with rapids, all axes when none is written. Nothing is invented: when the position is not configured the block moves nothing and is told with `reference-position-unset`.
* A `G92` offset larger than `max_offset_mm` (100000 by default) is refused with `offset-out-of-range`; `G92` with no axis word is `g92-axes-missing`.
* Every value (reference positions, work origins, the limit) belongs in the machine configuration of the program that calls the reader; none is in this crate.

## What is not done

* Cutter radius compensation (`G41`/`G42`) and canned cycles (`G81`..`G89`) are errors, and the block moves nothing.
* Tool length offsets and probing cycles are read but their effect on the position is not tracked.
* The position before the first move is taken as X0 Y0 Z0 of the work coordinate system the program uses; the reader cannot know where the machine really is, so a table limit is only as good as that assumption. `G92`, `G28`, `G30`, `G53`, `G10` and the probing cycles leave the position as it was and are told with the warning `position-untracked`, unless the program is walked with `interpret_coordinates` (next section).
* Planes other than XY for arcs (`G18` and `G19` are an error and the arc is drawn as a line).
* HP-GL and the Bungard format are planned.
* The pattern offset of Excellon (`M02X..`) and a drill file without a header whose format has to be guessed are not read: the first is an error so that no hole is invented, the second is read with the format of the caller (`ExcellonDefaults`).
* None of this has been tried on a collection of real production files, and nothing here decides whether a program is safe to run on a machine.

## The modal codes of a program

`eh3d_parser_gcode::ModalState` is the one tracker of the codes that stay in force (the unit, the distance mode, the plane, the work coordinate system, the feed mode, the feed, the spindle with its speed and the coolant), and the one that gives them back as lines to take a program up again from the middle. EH3D-SERVER uses it for that. The other programs of the suite do not need a second tracker: EH3D-CAM-DEBUGGER reads the position and the offsets with `interpret` and `interpret_coordinates` of this crate, and EH3D-AUTOLEVEL does not follow modes at all, because it refuses what it cannot rewrite (relative distances, inches and arcs) with the number of the line. A program that is not in this workspace (the machine program on the controller) keeps its own and is not part of the suite.
