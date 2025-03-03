use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::statistic::objects::logging_data::DateValue;
use crate::statistic::statistic::StatisticBase;
use anyhow::Result;
use chrono::NaiveDate;

pub trait IntegerStatistic: StatisticBase {
    fn get_integer_data_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<i64>>>>;
    fn get_integer_goals_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<i64>>>>;

    fn get_y_axis_conversion(&self) -> YAxisConversion;
}

// fn convert_to_int(&self, value: Self::RetrieveValue) -> i64;
// fn get_integer_data_closed_range(&self, start_date: NaiveDate, end_date: NaiveDate) -> anyhow::Result<Vec<LoggingData<i64>>> {
//     Ok(self.get_data_for_closed_range(start_date, end_date)?
//         .into_iter()
//         .map(|data| {
//             LoggingData {
//                 statistic_id: data.statistic_id,
//                 date: data.date,
//                 value: data.value.map(|value| self.convert_to_int(value))
//             }
//         })
//         .collect())
// }
