# The API other programs can build on

This page is the list of what a program that reads Gerber and Excellon files with this repository can rely on. Everything here is documented in the code (`cargo doc`), tested, and has no dependency: the crates use only the standard library.

## The two steps

1. **Read.** `eh3d_parser_gerber::parse(text)` (or `parse_bytes`, `parse_with_limits`, `parse_bytes_with_limits`) gives a `GerberFile`: the units, the coordinate format, the apertures, the macros, the attributes, the image parameters, the dialect, the commands in the order of the file and the diagnostics. `eh3d_parser_excellon::parse(text)` gives an `ExcellonFile`: the tools, holes, slots, routes (with their arcs), attributes, notes of the CAD program, dialect and diagnostics. Neither fails: what could not be understood is a `Diagnostic` with its line and a stable code.
2. **Put in place.** `eh3d_parser_gerber::image::resolve(&file)` returns an `Image`: the flat list of what is drawn, with every block aperture expanded, every copy of a step and repeat made, the transforms of the apertures and of the whole image applied, the arcs worked out (`arc::arc_geometry`) and the regions as contours. A program that draws or machines a layer needs only this list and the apertures of the file.

```rust
let file = eh3d_parser_gerber::parse(&text);
let image = eh3d_parser_gerber::image::resolve(&file);
for item in &image.items {
    match &item.shape {
        Shape::Flash { aperture, transform } => { /* the aperture, placed by the transform */ }
        Shape::Stroke { aperture, from, to, pen } => { /* a line drawn with the pen of the aperture */ }
        Shape::ArcStroke { aperture, from, to, center, clockwise, pen } => { /* an arc */ }
        Shape::Region { contours } => { /* a filled region, with holes as further contours */ }
    }
}
```

An item says its polarity and its group. The items of one group (one flash of a block aperture) are composed first and the result is added with the polarity the group was flashed with; an item with no group belongs to the layer. A negative image (`Image::negative`) means the items are the clear part of the layer.

## What each part gives

| Part | Public items | Notes |
|---|---|---|
| `eh3d-parser-common` | `Severity`, `Diagnostic`, `codes`, `Units`, `decode_coordinate`, `InputLimits`, `i18n` (`Language`, `headline`, `localize`), `dialect` (`Dialect`, `detect`) | The headline of every diagnostic code in eight languages. |
| `eh3d-parser-gerber` | `parse*`, `GerberFile`, `Operation`, `Aperture` (circle, rectangle, obround, polygon, macro, block), `ParseLimits` | Block apertures, `SR`, `LM`/`LR`/`LS`, `IP`/`OF`/`SF`/`MI`/`IR`/`AS`, incremental coordinates and the decimal comma are read. |
| `eh3d-parser-gerber::macros` | `evaluate`, `Primitive` (1, 2, 4, 5, 6, 7, 20, 21, 22), `bounds`, `eval_expression` | Variables and arithmetic evaluated; the shapes are not calculated here. |
| `eh3d-parser-gerber::image` | `resolve`, `resolve_with`, `Image`, `Item`, `Shape`, `Group`, `Affine`, `aperture_bounds` | Limits in `ResolveLimits`. |
| `eh3d-parser-gerber::attributes` | `file_attributes`, `aperture_functions`, `object_attributes`, `FileFunction`, `AperFunction`, `ObjectAttribute` | X2 and X3 as types; unknown values are kept as `Other`. |
| `eh3d-parser-gerber::dialect` | `detect`, `layer_from_filename`, `is_drill_filename` | Altium, Allegro, EAGLE, KiCad. |
| `eh3d-parser-gerber::arc` | `arc_geometry`, `ArcGeometry::flatten` | Single and multi quadrant. |
| `eh3d-parser-gerber::snapshot` | `normalize`, `micro` | The normalised text the corpus keeps. |
| `eh3d-parser-excellon` | `parse*`, `ExcellonFile`, `Route` and `RouteArc`, `ExcellonLimits`, `plating()` | `snapshot::normalize` for the corpus. |
| `eh3d-geometry` | `polygon` (`Region`, area, centroid, simplification), `boolean`, `offset`, `arcfit` | No dependency; the rounding of the booleans is documented in the module. |

## Promises

* **No panic, bounded work.** For any input the readers return, and the work is bounded by the size of the input and by the limits (`ParseLimits`, `ExcellonLimits`, `ResolveLimits`, `BooleanOptions::max_edges`). A limit that is reached stops the work with a diagnostic that names it.
* **Same input, same output.** There is no clock, no random number, no language of the machine and no state kept between two calls. The snapshot text of a reading is the same on every machine.
* **Units.** The readers keep the units of the file (`Image::units`, `GerberFile::units`). A program converts at the edge with `Units::to_mm`.
* **Codes.** A diagnostic code is never renamed or reused (see [DIAGNOSTICS.md](DIAGNOSTICS.md)). Filter on the code; the English text may be improved. `eh3d_parser_common::i18n::headline` gives the headline of a code in English, Spanish, German, French, Italian, Japanese, Chinese and Portuguese of Portugal.
* **Changes.** While the version starts with 0, a change that breaks a signature or the meaning of a field is written in the CHANGELOG under its own heading. The corpus snapshots (`tests/corpus/`) show every change of what a reader understands in a diff; a program that moves to these crates keeps the same snapshots (see [MIGRATION.md](MIGRATION.md)).

## What a program must still do itself

* Draw the apertures: the readers give the aperture of each flash and stroke, not a polygon. `eh3d-geometry::scene` has a simple conversion to shapes; a viewer that needs holes in apertures, clear polarity or the full macros makes its own from `Image` and `macros::evaluate`.
* Compose the polarity (dark and clear, and the groups).
* Decide what to do with a file that has errors: `GerberFile::is_clean()` and the diagnostics say what was lost.
