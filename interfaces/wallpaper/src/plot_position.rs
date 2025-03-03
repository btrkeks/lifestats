pub struct PlotPosition {
    wallpaper_nr: u8, // Starts at 1
    x: usize,
    y: usize,
}

impl PlotPosition {
    pub fn new(wallpaper_nr: u8, x: usize, y: usize) -> PlotPosition {
        debug_assert!(wallpaper_nr > 0, "Wallpaper number must be greater than 0");

        PlotPosition { wallpaper_nr, x, y }
    }

    pub fn get_wallpaper_nr(&self) -> u8 {
        self.wallpaper_nr
    }

    /// Returns the left right corner of the Plot in pixel coordinates
    pub fn get_pixel_coordinates(&self) -> (usize, usize) {
        // TODO: Make this configurable
        (self.x, self.y + 30) // Add some margin
    }
}
