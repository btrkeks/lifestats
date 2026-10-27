use plot_domain::plot::axis::AxisLabelFormat;
use serde::{Deserialize, Serialize};

/// Existing units retain integer JSON; scaled units expose human-unit decimals.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlotNumber {
    Integer(i64),
    Decimal(f64),
}

impl PlotNumber {
    pub fn for_axis(value: i64, format: AxisLabelFormat) -> Self {
        match format {
            AxisLabelFormat::KilogramsOneDecimal => Self::Decimal(value as f64 / 1000.0),
            AxisLabelFormat::PercentOneDecimal => Self::Decimal(value as f64 / 10.0),
            _ => Self::Integer(value),
        }
    }
}
