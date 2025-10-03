use plotters::prelude::{RGBAColor, RGBColor};

pub(crate) const LIGHT_BLACK: RGBColor = RGBColor(37, 37, 37);
pub(crate) const BACKGROUND_COLOR: &RGBColor = &LIGHT_BLACK;
pub(crate) const GOAL_LINE_COLOR: RGBAColor = RGBAColor(255, 0, 0, 0.7);
pub const SHADING_COLOR: RGBAColor = RGBAColor(200, 200, 200, 0.1);
pub const Y_MARGIN_PERCENT: i64 = 10;
