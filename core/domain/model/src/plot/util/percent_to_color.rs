use crate::plot::objects::color;
use crate::plot::objects::color::Color;

pub fn percent_to_color(opt_perc: Option<f64>) -> Color {
    match opt_perc {
        None => color::WHITE,
        Some(perc) => {
            if perc >= 1.5 {
                color::GOLD
            } else if perc >= 1.0 {
                color::GREEN
            } else if perc >= 0.9 {
                color::ORANGE
            } else {
                color::RED
            }
        }
    }
}
