// EH3D-CORE-LIBS - eh3d-raster.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! Raster geometry shared by the programs of the suite: units of length (`units`), the exact Euclidean distance from every cell of a grid to the nearest
//! occupied cell (`edt`), and the closed lines where that distance has a given value, simplified and oriented (`contour`).

pub mod contour;
pub mod edt;
pub mod units;
