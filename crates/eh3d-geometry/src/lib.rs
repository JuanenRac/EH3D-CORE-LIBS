// EH3D-CORE-LIBS - eh3d-geometry.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! The shapes of a layer in millimetres, with y up as in the file (`scene`): the `Shape` kinds, the `Scene` that holds them and the notes of what was not drawn, the
//! `Bounds` of a set of shapes, and the conversion of Gerber layers and Excellon drill files into scenes. The viewer and the CAM programs of the suite share these types.

mod macros;
pub mod scene;
