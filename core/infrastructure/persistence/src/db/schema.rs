// @generated automatically by Diesel CLI.

diesel::table! {
    default_goals (statistic_id) {
        statistic_id -> Integer,
        goal -> Text,
    }
}

diesel::table! {
    goals (statistic_id, date) {
        statistic_id -> Integer,
        date -> Text,
        goal -> Nullable<Text>,
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
    integrations (id) {
        id -> Integer,
        source_name -> Text,
        access_token -> Nullable<Text>,
        refresh_token -> Nullable<Text>,
        expires_at_epoch_seconds -> Nullable<BigInt>,
        scopes -> Nullable<Text>,
        last_sync_at -> Nullable<Text>,
        canonical_tags -> Nullable<Text>,
    }
}

diesel::table! {
    statistics (id) {
        id -> Integer,
        name -> Text,
        category -> Text,
        statistic_type -> Text,
        default_goal_valid_since -> Text,
        tags -> Text,
        canonical_tags -> Nullable<Text>,
    }
}

diesel::joinable!(default_goals -> statistics (statistic_id));
diesel::joinable!(goals -> statistics (statistic_id));

diesel::allow_tables_to_appear_in_same_query!(
    default_goals,
    goals,
    integrations,
    logging_data,
    statistics,
);
