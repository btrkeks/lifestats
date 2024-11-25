

#[derive(Copy, Clone)]
pub enum Color {
    Green,
    Red,
    White,
    Orange,
    RGB(u8, u8, u8),
}

impl Color {
    pub fn into_rgb(self) -> (u8, u8, u8) {
        match self { //TODO
            Color::Green => (0,0,0),
            Color::Red => (0,0,0),
            Color::White => (0,0,0),
            Color::Orange => (0,0,0),
            Color::RGB(r, g, b) => (r, g, b),
        }
    }
}