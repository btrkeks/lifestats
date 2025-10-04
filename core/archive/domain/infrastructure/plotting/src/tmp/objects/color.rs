pub const GREEN: Color = Color([169, 252, 177]);
pub const GOLD: Color = Color([255, 215, 0]);
pub const RED: Color = Color([245, 169, 142]);
pub const ORANGE: Color = Color([255, 174, 113]);
pub const LIGHT_BLACK: Color = Color([37, 37, 37]);
pub const WHITE: Color = Color([255, 255, 255]);
pub const GREY: Color = Color([211, 211, 211]);

#[derive(Debug, Copy, Clone)]
pub struct Color(pub(crate) [u8; 3]);

impl Color {
    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Color([r, g, b])
    }

    pub fn get_rgb(&self) -> (u8, u8, u8) {
        (self.0[0], self.0[1], self.0[2])
    }
}
