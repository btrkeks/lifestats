use crate::objects::date_value::DateValue;
use crate::statistic::traits::base_statistic::Statistic;
use chrono::NaiveDate;

pub trait IntegerStatistic: Statistic {
    fn value_into_i64(&self, value: Self::LogValueType) -> i64;

    fn get_integer_data_closed_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Vec<DateValue<i64>> {
        // TODO: Probably needs to be Option<i64>
        self.get_values_closed_range(start, end)
            .into_iter()
            .map(|date_value| {
                DateValue::new(date_value.date, self.value_into_i64(date_value.value))
            })
            .collect()
    }

    fn get_integer_goals_closed_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Vec<DateValue<i64>> {
        self.get_goals_closed_range(start, end)
            .into_iter()
            .map(|date_value| {
                DateValue::new(date_value.date, self.value_into_i64(date_value.value))
            })
            .collect()
    }
}
