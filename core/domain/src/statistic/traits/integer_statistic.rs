use crate::statistic::traits::base_statistic::Statistic;

pub trait IntegerStatistic: Statistic {
    fn value_into_i64(&self, value: Self::LogValueType) -> i64;

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
