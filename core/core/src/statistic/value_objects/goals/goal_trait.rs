use crate::shared::percentage::Percentage;

pub trait Goal: Clone {
    type LogValue;

    /// This accepts an Option as 'not logged values' might be handled differently depending on the goal
    /// E.g. we treat a not logged bad habit as 100% achieved
    fn get_percentage_achieved(&self, value: Option<&Self::LogValue>) -> Percentage;

    fn is_achieved(&self, value: Option<&Self::LogValue>) -> bool {
        self.get_percentage_achieved(value).as_num() >= 100
    }
}
