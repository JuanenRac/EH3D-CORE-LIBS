# What each reader promises

All three readers (`eh3d-parser-gerber`, `eh3d-parser-excellon`, `eh3d-parser-gcode`) share the same rules.

## The rules

* **No panic.** Any text, however damaged, comes back as a result. The tests feed them empty files, binary garbage, huge numbers, unterminated comments and lines of thousands of characters.
* **No guess.** A value that does not fit its declared format is refused (`diagnostic`), not rounded or truncated into something that looks right.
* **Diagnostics.** Each problem is a `Diagnostic { line, severity, message }`: `Info` (worth knowing), `Warning` (probably still right) or `Error` (something was lost). `is_clean()` is true when there is no `Error`.
* **Bounded.** The G-code reader cuts a line at 512 characters and says so.

## Gerber (RS-274X)

Units, the coordinate format (integer and decimal digits, leading or trailing zero suppression), apertures (circle, rectangle, obround, polygon) and macros, regions, and the drawing commands. A coordinate with more digits than its format allows is an error.

## Excellon

Tools with diameter, feed and speed; drill hits; slots with `G85`; simple routes (`M15`, `G01`, `M16`); metric or inch; `TZ` (leading zeros suppressed) or `LZ` (trailing zeros suppressed); coordinates with a decimal point are read as they are. `FILE_FORMAT=i:d` comments are honoured.

## G-code

Dialects: GRBL, Marlin (line numbers and checksums, verified) and PlanetCNC. `interpret` walks the program the way a controller does: `G20`/`G21`, `G90`/`G91`, `G90.1`/`G91.1`, `G0`..`G3` (arcs with `I`/`J` or `R`), `F`, `S`, `T`, `M3`/`M4`/`M5`, `M2`/`M30`. Every motion comes out in absolute millimetres, with the feed in millimetres per minute.

## What is not done

* Cutter radius compensation (`G41`/`G42`) and canned cycles (`G81`..`G89`) are errors, and the block moves nothing.
* Tool length offsets, probing cycles, `G28`/`G30` and work coordinate systems are read but their effect on the position is not tracked.
* Planes other than XY for arcs.
* HP-GL and the Bungard format, and the 2D geometry (offsets and booleans), are planned.
* None of this has been tried on a collection of real production files, and nothing here decides whether a program is safe to run on a machine.
