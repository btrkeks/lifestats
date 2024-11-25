use crate::objects::y_axis_conversion::YAxisConversion;

pub trait SingleIntegerPlottable {
    fn get_value(&self) -> i64;
    fn get_goal(&self) -> i64;
    fn y_unit(&self) -> String;
    fn y_axis_converter(num: &i64) -> YAxisConversion;
}