use crate::value_objects::date_range::DateRange;
const DEFAULT_PLOT_RANGE: i64 = 14;
const DEFAULT_PLOT_SIZE: (usize, usize) = (450, 200);

pub struct PlotConfig {
    pub size: (usize, usize),
    pub date_range: DateRange
}

impl PlotConfig {
    pub fn default() -> Self {
        let end = chrono::Local::now().date_naive();
        let start = end - chrono::Duration::days(DEFAULT_PLOT_RANGE);

        Self {
            size: DEFAULT_PLOT_SIZE,
            date_range: DateRange::new(start, end).unwrap()
        }
    }

    pub fn size(&self) -> (usize, usize) {
        self.size
    }
}