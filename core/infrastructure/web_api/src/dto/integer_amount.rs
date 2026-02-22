use lifestats_application::use_cases::queries::get_integer_amount_total::IntegerAmountTotalResult;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct IntegerAmountTotalResponse {
    pub total: i64,
}

impl From<IntegerAmountTotalResult> for IntegerAmountTotalResponse {
    fn from(result: IntegerAmountTotalResult) -> Self {
        Self {
            total: result.total,
        }
    }
}
