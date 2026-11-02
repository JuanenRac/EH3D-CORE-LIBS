// EH3D-CORE-LIBS - eh3d-cam.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! The CAM that needs nothing but geometry. The copper of a layer is drawn on a fine grid (`bitmap`), the exact distance of every point of the board to the copper is worked out, and
//! the paths of the tool are the lines where that distance equals the radius of the tool (`isolate`, with `forced` and `fill` for the passes that cut more). Around them: the checks
//! of the gaps the tool cannot cut (`problems`, `tolerance`, `drc`), the order and the shortening of the paths (`order`, `optimize`), the alignment on fiducials (`fiducial`),
//! the mirror, rotation and layout of a board (`transform`), solder mask openings (`mask`) and old-style Gerber (`legacy`). Nothing here reads a file from disk or writes G-code.

pub mod bitmap;
pub mod drc;
pub mod fiducial;
pub mod fill;
pub mod forced;
pub mod hole;
pub mod isolate;
pub mod legacy;
pub mod mask;
pub mod optimize;
pub mod order;
pub mod problems;
pub mod tolerance;
pub mod transform;
