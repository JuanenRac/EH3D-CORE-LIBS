# What each reader promises

All three readers (`eh3d-parser-gerber`, `eh3d-parser-excellon`, `eh3d-parser-gcode`) share the same rules.

## The rules

* **No panic.** Any text, however damaged, comes back as a result. The tests feed them empty files, binary garbage, huge numbers, unterminated comments and lines of thousands of characters.
* **No guess.** A value that does not fit its declared format is refused (`diagnostic`), not rounded or truncated into something that looks right.
* **Diagnostics.** Each problem is a `Diagnostic { line, severity, code, message }`: `Info` (worth knowing), `Warning` (probably still right) or `Error` (something was lost). `is_clean()` is true when there is no `Error`. The `code` is a stable identifier of the kind of notice (see [DIAGNOSTICS.md](DIAGNOSTICS.md)): filter on it, not on the text.
* **Shared code.** `Severity`, `Diagnostic`, `Units`, the decoding of coordinates and the limits on numbers live in the internal crate `eh3d-parser-common`; the readers re-export them, so a program only depends on the reader it uses.
* **Files.** ead_file_limited refuses a file above the limit by its size on disk, before it is read into memory.
* **Bytes.** `parse_bytes` reads a file from its bytes with a size limit (256 MiB by default) and reads text that is not UTF-8 as Latin-1, with an `input-not-utf8` warning.
* **Units.** A Gerber or Excellon file that does not declare its units is read in millimetres, with a `units-missing` warning.
* **Bounded.** The G-code reader cuts a line at 512 characters and says so. A number above 1e15 is refused with `number-too-large`. A program keeps at most 5000000 blocks (`program-too-long`) and every reader at most 100000 diagnostics (`too-many-diagnostics`); `parse_with_limits` sets other limits.

## Gerber (RS-274X)

Units, the coordinate format (integer and decimal digits, leading or trailing zero suppression), apertures (circle, rectangle, obround, polygon) and macros, regions, and the drawing commands. A coordinate with more digits than its format allows is an error. A turn, a mirror or a scale of the apertures (LR, LM, LS) and aperture blocks (AB) are not applied, and are an error when they would change the drawing. The reader keeps at most 100000 diagnostics per file.

## Excellon

Tools with diameter, feed and speed; drill hits; slots with `G85`; simple routes (`M15`, `G01`, `M16`); metric or inch; `TZ` (leading zeros suppressed) or `LZ` (trailing zeros suppressed); coordinates with a decimal point are read as they are. `FILE_FORMAT=i:d` comments are honoured.

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
* HP-GL and the Bungard format, and the 2D geometry (offsets and booleans), are planned.
* None of this has been tried on a collection of real production files, and nothing here decides whether a program is safe to run on a machine.
