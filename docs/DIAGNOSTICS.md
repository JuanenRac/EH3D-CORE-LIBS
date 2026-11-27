# Diagnostic codes and the units policy

Every [`Diagnostic`] has a `line`, a `severity`, a `message` (English text, free to improve) and a `code`: a short, stable identifier of the kind of notice. A program that wants to react to one kind of notice (hide a deprecation, stop on a missing unit) compares the **code**, not the text and not the severity. A code is never renamed or reused; a new kind of notice gets a new code.

The codes are the constants of `eh3d_parser_common::codes` (re-exported by the three readers as `codes`). The test `docs_list_every_code` fails when this table and the constants differ.

## The codes

| Code | Readers | Severity | Meaning |
|---|---|---|---|
| `units-missing` | Gerber, Excellon | Warning | The file does not declare its units; millimetres are assumed (see below). |
| `number-too-large` | all | Error | A number is above the largest the readers accept (1e15), or a coordinate has more digits than its format. |
| `coordinate-invalid` | Gerber, Excellon | Error | The digits are not a coordinate. |
| `coordinate-before-format` | Gerber | Error | A coordinate comes before the `FS` command. |
| `unknown-g-code` | all | Warning / Error | A `G` code the reader does not know; ignored. In G-code it is an error when the line has axis words (they are not read as a move) or the number is not a code at all. |
| `unknown-m-code` | all | Warning / Error | An `M` code the reader does not know; ignored. In G-code a number that is not a whole one is an error. |
| `unknown-word` | all | Warning / Error | A letter and number the reader does not know; ignored. In G-code it is an error where the controller refuses the line: a word `E` for GRBL, or axis words after `G80` with no new motion code. |
| `end-missing` | Gerber, Excellon | Warning | The file does not end with `M02` / `M30`. |
| `text-after-end` | Excellon, G-code, Gerber | Warning / Error | Text or blocks after the end of the program. An error in G-code: a controller runs what it is sent and it was not looked at. In Gerber the commands after `M02` are still read. |
| `incremental-unsupported` | G-code, Excellon | Error | Incremental coordinates that the reader cannot follow (G-code `G91`; in Excellon, an `ICI` that is neither `ON` nor `OFF`). The Gerber and Excellon readers follow `G91`, `FS ... I` and `ICI,ON` with a `deprecated-command` warning. |
| `deprecated-command` | Gerber, Excellon | Info or Warning | A deprecated command that was ignored or read as its replacement (in Gerber also the image parameters `IP`, `OF`, `SF`, `MI`, `IR`, `AS` and incremental coordinates; in Excellon `G91` and `ICI,ON`). |
| `command-ignored` | Gerber, Excellon | Info or Warning | A known command with no effect on the result was ignored. In Excellon also a zero set (`G92`, `G93`) at zero, and the position written on the end line (`M00`, `M30` with X or Y), which is not a hole (Warning). |
| `too-many-diagnostics` | Gerber, Excellon | Error | The file has more diagnostics than the reader keeps (100000); the rest are dropped. The notices `format-assumed`, `zeros-assumed` and the limit that stopped the reading are kept anyway. |
| `command-unsupported` | Gerber | Warning | A known command with no effect on the geometry that was ignored. |
| `geometry-command-unsupported` | Gerber, Excellon | Error | A command that changes the geometry and is not supported (in Excellon, a repeat `R<n>`, a pattern offset `M02` with a coordinate, or a zero set `G92`/`G93` away from zero): in Gerber, the deprecated knock-out (`KO`) and plot film (`PF`) commands. |
| `unknown-command` | Gerber | Warning | An extended command the reader does not know. |
| `input-not-utf8` | all (`parse_bytes`) | Warning | The bytes are not UTF-8 and were read as Latin-1. |
| `extended-unclosed` | Gerber | Error | An extended command starting with `%` is never closed. |
| `block-unclosed` | Gerber | Warning | A block is not closed by `*`. |
| `region-unclosed` | Gerber | Error | A region (`G36`) is never closed by `G37`. |
| `region-nested` | Gerber | Error | `G36` inside a region. |
| `region-not-open` | Gerber | Error | `G37` without `G36`. |
| `macro-name-missing` | Gerber | Error | An aperture macro has no name. |
| `units-unknown` | Gerber | Error | The units of `MO` are not `MM` or `IN`. |
| `polarity-unknown` | Gerber | Error | The polarity of `LP` is not `D` or `C`. |
| `format-zero-omission-invalid` | Gerber | Error | `FS`: the zero omission is not `L` or `T`. |
| `format-trailing-deprecated` | Gerber | Warning | `FS`: omitting trailing zeros is deprecated. |
| `format-mode-invalid` | Gerber | Error | `FS`: the coordinate mode is not absolute. |
| `format-malformed` | Gerber | Error | `FS`: `X<digits>Y<digits>` was expected. |
| `format-axes-differ` | Gerber | Warning | `FS`: X and Y use different formats; X is used for both. |
| `format-digits-invalid` | Gerber | Error | `FS`: the format is not two digits in range. |
| `aperture-number-invalid` | Gerber | Error | `AD`: the aperture number is missing or does not start with `D`. |
| `aperture-number-reserved` | Gerber | Error | `AD`: the number is reserved (below `D10`). |
| `aperture-parameter-invalid` | Gerber | Error | `AD`: a parameter is not a number. |
| `aperture-parameters-missing` | Gerber | Error | `AD`: the template has too few parameters. |
| `aperture-polygon-vertices` | Gerber | Error | `AD`: a polygon has 3 to 12 vertices. |
| `aperture-macro-undefined` | Gerber | Warning | `AD`: the macro is not defined before its use. |
| `aperture-template-missing` | Gerber | Error | `AD`: no template. |
| `aperture-size-negative` | Gerber | Warning | `AD`: a negative size. |
| `aperture-redefined` | Gerber | Warning | `AD`: the aperture is defined twice; the last one wins. |
| `aperture-not-selected` | Gerber | Error | A flash or a draw comes before any aperture is selected. |
| `aperture-undefined` | Gerber | Error or Warning | An aperture is used or selected without a definition. |
| `step-repeat-invalid` | Gerber | Error | `SR` with a bad value or without X, Y, I and J. |
| `block-empty` | Gerber | Warning | A block has no letters. |
| `d-code-invalid` | Gerber | Error or Warning | A `D` code is not an operation or an aperture. |
| `implied-operation` | Gerber | Warning | Coordinates without `D01`/`D02`/`D03`; the last operation was used. |
| `operation-missing` | Gerber | Error | Coordinates without an operation and none to repeat. |
| `coordinate-missing` | Gerber, Excellon | Error | An operation uses a coordinate that was never given; in Excellon, a slot (`G85`) without its end. |
| `start-point-missing` | Gerber | Error | A draw starts before any move gave a start point. |
| `limit-exceeded` | Gerber, Excellon | Error | A limit of the reading was reached (`ParseLimits`: commands, apertures, macros, attributes, nesting of block apertures, copies of a step and repeat); the reading stops there and returns what it has, except for a step and repeat over its copies, which is only not applied. |
| `block-aperture-invalid` | Gerber | Error | A block aperture (`AB`) is closed without being opened, is opened with a number below 10, or is never closed; also a draw with a block aperture. |
| `block-aperture-cycle` | Gerber | Error | A block aperture draws itself, directly or through another, or is nested deeper than the limit; reported when the drawing is put in place (`image::resolve`) and the flash is left out. |
| `decimal-comma` | Gerber, Excellon | Warning | A number of an aperture or a coordinate uses a comma as the decimal point (a file written by a computer set to a language that writes it so); it is read as a point. |
| `attribute-malformed` | Gerber | Warning | An attribute (`TF`, `TA`, `TO`) has no name. |
| `layer-transform-invalid` | Gerber | Error | The value of `LM` (not `N`, `X`, `Y` or `XY`), `LR` (not a number) or `LS` (not a number above zero) cannot be read. |
| `image-parameter-invalid` | Gerber | Error | A deprecated parameter of the whole image (`IP`, `OF`, `SF`, `MI`, `IR`, `AS`) has a value that cannot be read. |
| `image-too-large` | Gerber | Error | The drawing put in place has more objects than the limit (`ResolveLimits::max_items`); the rest is left out. |
| `header-missing` | Excellon | Warning | The file has no `M48` header. |
| `format-comment-invalid` | Excellon | Warning | A `FILE_FORMAT=` comment is not understood. |
| `defaults-invalid` | Excellon | Warning | The default format given to `parse_with_format` is out of range (1 to 8 integer digits, 0 to 8 decimal digits) and was ignored. |
| `option-unknown` | Excellon | Warning | An unknown option after `METRIC` / `INCH`. |
| `unknown-header-line` | Excellon | Warning / Error | A header line the reader does not know. An error when it is a coordinate: the header was not closed and the hole is not drilled. |
| `tool-number-missing` | Excellon | Error | A tool definition has no number. |
| `tool-diameter-invalid` | Excellon | Error | A tool diameter is not a positive number. |
| `tool-diameter-missing` | Excellon | Error | A tool is defined without a diameter. |
| `tool-parameter-unknown` | Excellon | Warning | An unknown tool parameter was ignored. |
| `tool-redefined` | Excellon | Warning | A tool is defined twice; the last one wins. |
| `tool-undefined` | Excellon | Error | A tool, hole or slot uses a tool that was never defined. |
| `tool-selection-invalid` | Excellon | Error | A tool selection has no number. |
| `tool-not-selected` | Excellon | Error | A hole or a slot comes before any tool is selected. |
| `arc-unsupported` | Excellon | Error | An arc (`G02`/`G03`) outside a route (the tool is not down, no `M15`): it is not drawn. Arcs inside a route are read (see `Route::arcs`). |
| `line-too-long` | G-code | Warning / Error | The line is longer than the limit and was cut. An error when code stood past the limit (it was not looked at); a warning when only a comment did. |
| `block-delete-read` | G-code | Info | A block-delete line was read as if the switch were off. |
| `system-command-ignored` | G-code | Info / Error | A GRBL `$` command was left out. The ones that only ask (`$`, `$$`, `$#`, `$G`, `$I`, `$N`) are a note; every other one can move the machine or change its settings and is an error. |
| `checksum-mismatch` | G-code | Error | The Marlin checksum of the line does not match. |
| `checksum-invalid` | G-code | Error | The Marlin checksum is not a number. |
| `comment-unclosed` | G-code | Warning | A comment in parentheses is not closed. |
| `word-number-invalid` | G-code | Error | A word has no valid number. |
| `character-ignored` | G-code, Gerber, Excellon | Error or Warning | A character that is not part of the language was left out, so the line was not read as written (in G-code, a line with only `%`, the start and end of a program, is not reported). In Gerber and Excellon, what stands before the first letter of a block or line (a warning in Excellon when it holds no digit). |
| `plane-unsupported` | G-code | Error | Only the XY plane is supported for arcs: after `G18` or `G19` an arc is drawn as a line. |
| `cutter-compensation-unsupported` | G-code | Error | `G41` / `G42` are not supported. |
| `canned-cycle-unsupported` | G-code | Error | Canned cycles are not supported. |
| `position-untracked` | G-code | Warning | A command was read but its effect on the position is not tracked. |
| `negative-feed` | G-code | Error | A negative feed rate was ignored. |
| `negative-spindle` | G-code | Error | A negative spindle speed was ignored. |
| `tool-number-invalid` | G-code | Error | A tool number that is not a whole number was ignored. |
| `implied-rapid` | G-code | Warning | An axis word before any `G0`-`G3` was taken as a rapid. |
| `arc-radius-unreachable` | G-code | Error | An arc with a radius that does not reach was drawn as a line. |
| `arc-radius-mismatch` | G-code | Error | An arc given by its centre (`I`/`J`) whose radius at the start and at the end differ by more than a controller accepts (0.005 mm and 0.1 % of the radius, or 0.5 mm). |
| `program-too-long` | G-code | Error | The program has more blocks than the reader keeps (5000000 by default); the rest were not read. |
| `units-suspect` | Excellon | Warning | The file declares no units and the holes span less than 5 units: it may be in inches. |
| `units-forced` | Excellon | Warning | The caller gave units (`parse_with_units`) and the file declares other ones: the ones given are used. |
| `arc-centre-missing` | G-code, Gerber, Excellon | Error | An arc without centre and radius was drawn as a line (G-code), an arc of a route that has neither `A` nor `I`/`J` (Excellon) or an arc with no centre to speak of that was left out of the drawing (Gerber `image::resolve`). |
| `feed-missing` | G-code | Warning | A cutting move was made with no feed rate set. |
| `reference-position-unset` | G-code | Warning | `G28` or `G30` was read by `interpret_coordinates` and no reference position is configured for it: the block moves nothing and the position is not tracked. |
| `g53-motion-unsupported` | G-code | Error | `G53` was read with a motion other than `G0` or `G1`: the block moves nothing. |
| `g92-axes-missing` | G-code | Warning | `G92` was read with no axis word: the offset is unchanged. |
| `offset-out-of-range` | G-code | Error | A `G92` offset would be larger than the configured limit (`max_offset_mm`): the offset of that axis was not changed. |
| `realtime-character` | G-code | Warning | A comment holds a character that GRBL takes out of the stream and obeys at once (`!`, `~`, `?` or a control character). In the code the same character is `character-ignored`. Accented letters and the other characters outside ASCII are taken in a comment and refused outside it: a program that sends lines to a controller has to drop or clean the comments, because GRBL takes some bytes of such letters as commands. |
| `feed-mode-unsupported` | G-code | Error | `G93` (inverse time) or `G95` (per revolution) changes what `F` means, so the speeds cannot be told. |
| `word-repeated` | G-code, Gerber, Excellon | Error | A word written twice on a line, or two codes of the same group (`G0 G1`, `G20 G21`, `G90 G91`, `M3 M5`): the controller refuses the line. In Gerber (X, Y, I, J, D) and Excellon (X, Y, A, I, J) the last one is used. |
| `format-assumed` | Excellon | Warning | A coordinate without a point of a file that declares no format (`FILE_FORMAT` comment or number mask) was read with the format supposed: 2.4 in inches, 3.3 in millimetres. The holes may be at another scale. Given once, at the first such coordinate that is not zero. An error after `refuse_assumptions`. |
| `zeros-assumed` | Excellon | Warning | A coordinate with fewer digits than its format, in a file that declares no zero suppression (`TZ`, `LZ`), was read with the leading zeros left out. Given once. An error after `refuse_assumptions`. |
| `units-conflict` | Gerber, Excellon | Error | The file declares two different units; the last ones are used. |
| `format-redefined` | Gerber | Error | A second `FS` declares another format; the new one is used from there on. |

## When the file does not declare its units

One rule, `eh3d_parser_common::resolve_units`, used by the Gerber and the Excellon readers:

* A unit the file declares (`MO`, `G70`/`G71` in Gerber; `METRIC`, `INCH`, `M71`, `M72` in Excellon) is used as it is and there is no notice.
* When there is none, the file keeps `units: None` (the field does not change), `effective_units()` returns millimetres, and the reader adds a **Warning with the code `units-missing`**. The warning does not make the file "not clean" (`is_clean()` looks at errors only).
* Excellon reads the integer coordinates of such a file with the metric default format (3.3), as it did before; Gerber needs its `FS` format either way.
* The G-code reader has no such rule: it starts in millimetres, as G-code controllers do, and follows `G20` / `G21`.

Both readers did the same thing before (leave `units: None`, no notice); the only change is the notice and `effective_units()`.

**Open decision.** Real Excellon files from old tools that omit the unit were drawn in inches by some programs (the Excellon convention of the 1980s) and in millimetres by others. Options: (a) keep millimetres (today); (b) assume inches for Excellon only; (c) make the assumed unit a setting of the caller. Nothing changes until this is decided.

## When an Excellon file does not declare its format

Many drill files write integers (`X15000Y5000`) and say neither how many digits stand on each side of the point nor which zeros are left out. The reader still reads them, as before (2.4 in inches, 3.3 in millimetres, leading zeros left out), and says so with `format-assumed` and `zeros-assumed`. Both are warnings, so `is_clean()` does not change. A coordinate with a point, and a zero, mean the same under any format and give no notice; so does a coordinate with all the digits of its format as far as the zeros go. What the caller gives in `ExcellonDefaults` (`default_format`, `default_zeros`) is the choice of the caller and is not told.

A program that moves a machine with what it reads calls `ExcellonFile::refuse_assumptions()`: the two notices, and `units-missing` when there is something to place, become errors. `ExcellonFile::assumptions()` lists them to show them to a person. A caller that knows the format passes it to `parse_with_format`.

`parse_with_units` supposes the format of the units given for a file that declares none (2.4 for inches).

## Reading bytes

`parse_bytes(bytes)` (and `parse_bytes_with(bytes, &InputLimits)`) in each reader replaces `fs::read` plus `String::from_utf8_lossy` in the programs:

* Over `InputLimits::max_bytes` (256 MiB by default) the input is refused with `InputError::TooLarge` before it is looked at.
* UTF-8, with or without a byte order mark, is taken as it is.
* Anything else is read as Latin-1 (each byte is the character of the same number, so a `é` written by an old tool stays a `é`) and the first diagnostic is `input-not-utf8`. Open decision: Windows-1252 would read the bytes 0x80-0x9F as printable characters instead of control characters; it would make no difference to the files the readers understand, which are ASCII outside of comments.
