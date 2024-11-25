// @generated automatically by Diesel CLI.

diesel::table! {
    logging_data (statistic_id, date) {
        statistic_id -> Integer,
        date -> Text,
        value -> Nullable<Text>,
        goal -> Nullable<Text>,
    }
}
