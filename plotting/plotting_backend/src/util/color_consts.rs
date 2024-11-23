use plotters::style::RGBColor;

pub const GOLD: RGBColor = RGBColor(255, 215, 0);
pub const GREEN: RGBColor = RGBColor(169, 252, 177);
pub const RED: RGBColor = RGBColor(245, 159, 142);
pub const ORANGE: RGBColor = RGBColor(255, 174, 113);

// pub(crate) const ANKI_GREEN: RGBColor = RGBColor(169, 252, 177);
// pub(crate) const ANKI_RED: RGBColor = RGBColor(245, 159, 142);
pub(crate) const LIGHT_BLACK: RGBColor = RGBColor(37, 37, 37);
pub(crate) const BACKGROUND_COLOR: &RGBColor = &LIGHT_BLACK;
pub(crate) const GOAL_LINE_COLOR: RGBColor = plotters::prelude::RED;
