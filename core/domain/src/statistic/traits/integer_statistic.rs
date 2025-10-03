use crate::shared::dated_value::DatedValue;
use crate::statistic::traits::base_statistic::Statistic;
use chrono::NaiveDate;

pub trait IntegerStatistic: Statistic {
    fn value_into_i64(&self, value: Self::LogValueType) -> i64;

    fn get_integer_data_closed_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Vec<DatedValue<i64>> {
        self.get_log_values_closed_range(start, end)
            .into_iter()
            .map(|date_value| {
                DatedValue::new(date_value.date, self.value_into_i64(date_value.value))
            })
            .collect()
    }

    // TODO: This method needs to be redesigned to properly handle goals
    // fn get_integer_goals_closed_range(
    //     &self,
    //     start: NaiveDate,
    //     end: NaiveDate,
    // ) -> Vec<DatedValue<i64>> {
    //     self.get_goals_closed_range(start, end)
    //         .into_iter()
    //         .map(|date_value| {
    //             DatedValue::new(date_value.date, self.value_into_i64(date_value.value))
    //         })
    //         .collect()
    // }
}
