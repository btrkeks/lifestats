use crate::objects::percentage::Percentage;

pub trait Goal: Clone {
    type LogValue;
    fn get_percentage_achieved(&self, value: Option<&Self::LogValue>) -> Percentage;

    fn is_achieved(&self, value: Option<&Self::LogValue>) -> bool {
        self.get_percentage_achieved(value).as_num() >= 100
    }
}