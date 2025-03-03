// @generated automatically by Diesel CLI.

diesel::table! {
    default_goals (statistic_id) {
        statistic_id -> Integer,
        target_value -> Text,
        extra_data1 -> Nullable<Text>,
    }
}

diesel::table! {
    goals (statistic_id, date) {
        statistic_id -> Integer,
        date -> Text,
        target_value -> Nullable<Text>,
        extra_data1 -> Nullable<Text>,
    }
}

diesel::table! {
    logging_data (statistic_id, date) {
        statistic_id -> Integer,
        date -> Text,
        value -> Text,
    }
}

diesel::table! {
    plots (id) {
        id -> Integer,
        statistic_id -> Integer,
        title -> Text,
        width -> Integer,
        height -> Integer,
        plot_type -> Text,
    }
}

diesel::table! {
    statistics (id) {
        id -> Integer,
        name -> Text,
        category -> Text,
        statistic_type -> Text,
    }
}

diesel::joinable!(default_goals -> statistics (statistic_id));
diesel::joinable!(goals -> statistics (statistic_id));
diesel::joinable!(plots -> statistics (statistic_id));

diesel::allow_tables_to_appear_in_same_query!(
    default_goals,
    goals,
    logging_data,
    plots,
    statistics,
);
