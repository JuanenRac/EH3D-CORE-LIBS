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
| `unknown-g-code` | all | Warning | A `G` code the reader does not know; ignored. |
| `unknown-m-code` | all | Warning | An `M` code the reader does not know; ignored. |
| `unknown-word` | all | Warning | A letter and number the reader does not know; ignored. |
| `end-missing` | Gerber, Excellon | Warning | The file does not end with `M02` / `M30`. |
| `text-after-end` | Excellon, G-code | Warning | Text or blocks after the end of the program. |
| `incremental-unsupported` | all | Error | Incremental coordinates (`G91`, `ICI`, `FS ... I`) are not supported. |
| `deprecated-command` | Gerber | Info or Warning | A deprecated command that was ignored or read as its replacement. |
| `command-ignored` | Gerber, Excellon | Info | A known command with no effect on the result was ignored. |
| `too-many-diagnostics` | Gerber | Error | The file has more diagnostics than the reader keeps (100000); the rest are dropped. |
| `command-unsupported` | Gerber | Warning | A known command with no effect on the geometry (`LM`, `LR` or `LS` with the value that changes nothing) that was ignored. |
| `geometry-command-unsupported` | Gerber | Error | A command that changes the geometry and is not supported: a deprecated one (negative image, offset, scale, mirror, turn), a layer transform of the apertures (`LM`, `LR`, `LS` with a value that changes something) or an aperture block (`AB`). |
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
| `coordinate-missing` | Gerber | Error | An operation uses a coordinate that was never given. |
| `start-point-missing` | Gerber | Error | A draw starts before any move gave a start point. |
| `header-missing` | Excellon | Warning | The file has no `M48` header. |
| `format-comment-invalid` | Excellon | Warning | A `FILE_FORMAT=` comment is not understood. |
| `option-unknown` | Excellon | Warning | An unknown option after `METRIC` / `INCH`. |
| `unknown-header-line` | Excellon | Warning | A header line the reader does not know. |
| `tool-number-missing` | Excellon | Error | A tool definition has no number. |
| `tool-diameter-invalid` | Excellon | Error | A tool diameter is not a positive number. |
| `tool-diameter-missing` | Excellon | Error | A tool is defined without a diameter. |
| `tool-parameter-unknown` | Excellon | Warning | An unknown tool parameter was ignored. |
| `tool-redefined` | Excellon | Warning | A tool is defined twice; the last one wins. |
| `tool-undefined` | Excellon | Error | A tool, hole or slot uses a tool that was never defined. |
| `tool-selection-invalid` | Excellon | Error | A tool selection has no number. |
| `tool-not-selected` | Excellon | Error | A hole or a slot comes before any tool is selected. |
| `arc-unsupported` | Excellon | Error | Arcs in routes (`G02`/`G03`) are not supported. |
| `line-too-long` | G-code | Warning | The line is longer than the limit and was cut. |
| `block-delete-read` | G-code | Info | A block-delete line was read as if the switch were off. |
| `system-command-ignored` | G-code | Info | A GRBL `$` system command was ignored. |
| `checksum-mismatch` | G-code | Error | The Marlin checksum of the line does not match. |
| `checksum-invalid` | G-code | Warning | The Marlin checksum is not a number. |
| `comment-unclosed` | G-code | Warning | A comment in parentheses is not closed. |
| `word-number-invalid` | G-code | Error | A word has no valid number. |
| `character-ignored` | G-code | Warning | A character that is not part of the language was ignored. |
| `plane-unsupported` | G-code | Warning | Only the XY plane is supported for arcs. |
| `cutter-compensation-unsupported` | G-code | Error | `G41` / `G42` are not supported. |
| `canned-cycle-unsupported` | G-code | Error | Canned cycles are not supported. |
| `position-untracked` | G-code | Warning | A command was read but its effect on the position is not tracked. |
| `negative-feed` | G-code | Error | A negative feed rate was ignored. |
| `negative-spindle` | G-code | Error | A negative spindle speed was ignored. |
| `tool-number-invalid` | G-code | Error | A tool number that is not a whole number was ignored. |
| `implied-rapid` | G-code | Warning | An axis word before any `G0`-`G3` was taken as a rapid. |
| `arc-radius-unreachable` | G-code | Error | An arc with a radius that does not reach was drawn as a line. |
| `arc-centre-missing` | G-code | Error | An arc without centre and radius was drawn as a line. |
| `feed-missing` | G-code | Warning | A cutting move was made with no feed rate set. |

## When the file does not declare its units

One rule, `eh3d_parser_common::resolve_units`, used by the Gerber and the Excellon readers:

* A unit the file declares (`MO`, `G70`/`G71` in Gerber; `METRIC`, `INCH`, `M71`, `M72` in Excellon) is used as it is and there is no notice.
* When there is none, the file keeps `units: None` (the field does not change), `effective_units()` returns millimetres, and the reader adds a **Warning with the code `units-missing`**. The warning does not make the file "not clean" (`is_clean()` looks at errors only).
* Excellon reads the integer coordinates of such a file with the metric default format (3.3), as it did before; Gerber needs its `FS` format either way.
* The G-code reader has no such rule: it starts in millimetres, as G-code controllers do, and follows `G20` / `G21`.

Both readers did the same thing before (leave `units: None`, no notice); the only change is the notice and `effective_units()`.

**Open decision.** Real Excellon files from old tools that omit the unit were drawn in inches by some programs (the Excellon convention of the 1980s) and in millimetres by others. Options: (a) keep millimetres (today); (b) assume inches for Excellon only; (c) make the assumed unit a setting of the caller. Nothing changes until this is decided.

## Reading bytes

`parse_bytes(bytes)` (and `parse_bytes_with(bytes, &InputLimits)`) in each reader replaces `fs::read` plus `String::from_utf8_lossy` in the programs:

* Over `InputLimits::max_bytes` (256 MiB by default) the input is refused with `InputError::TooLarge` before it is looked at.
* UTF-8, with or without a byte order mark, is taken as it is.
* Anything else is read as Latin-1 (each byte is the character of the same number, so a `é` written by an old tool stays a `é`) and the first diagnostic is `input-not-utf8`. Open decision: Windows-1252 would read the bytes 0x80-0x9F as printable characters instead of control characters; it would make no difference to the files the readers understand, which are ASCII outside of comments.
