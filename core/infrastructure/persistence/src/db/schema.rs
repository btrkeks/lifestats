// @generated automatically by Diesel CLI.

diesel::table! {
    default_goals (statistic_id) {
        statistic_id -> Integer,
        goal -> Text,
    }
}

diesel::table! {
    default_goal_segments (statistic_id, valid_since) {
        statistic_id -> Integer,
        valid_since -> Text,
        goal -> Nullable<Text>,
    }
}

diesel::table! {
    ebook_sync_events (event_id) {
        event_id -> Text,
        source_name -> Text,
        metric -> Text,
        statistic_id -> Integer,
        date -> Text,
        delta -> BigInt,
        book_id -> Nullable<Text>,
        book_title -> Text,
        confirmed_position_after -> BigInt,
        created_at -> Text,
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
    progress_checkpoints (id) {
        id -> Integer,
        progress_item_id -> Integer,
        recorded_at -> Text,
        effective_date -> Text,
        position_value -> BigInt,
        source -> Nullable<Text>,
        source_event_id -> Nullable<Text>,
        note -> Nullable<Text>,
        created_at -> Text,
    }
}

diesel::table! {
    progress_items (id) {
        id -> Integer,
        name -> Text,
        category -> Text,
        unit_kind -> Text,
        unit_label -> Nullable<Text>,
        target_value -> BigInt,
        tags -> Text,
        canonical_tags -> Nullable<Text>,
        created_at -> Text,
        updated_at -> Text,
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

diesel::joinable!(default_goal_segments -> statistics (statistic_id));
diesel::joinable!(default_goals -> statistics (statistic_id));
diesel::joinable!(ebook_sync_events -> statistics (statistic_id));
diesel::joinable!(goals -> statistics (statistic_id));
diesel::joinable!(progress_checkpoints -> progress_items (progress_item_id));

diesel::allow_tables_to_appear_in_same_query!(
    default_goal_segments,
    default_goals,
    ebook_sync_events,
    goals,
    integrations,
    logging_data,
    progress_checkpoints,
    progress_items,
    statistics,
);
