//! One hole of a drill file, as the CAM modules handle it.

/// One hole: where, how wide and with which tool number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hole {
    pub tool: u8,
    pub x_mm: f64,
    pub y_mm: f64,
    /// 0 when the file does not say.
    pub diameter_mm: f64,
}
