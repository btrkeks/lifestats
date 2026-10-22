CREATE TABLE yomikata_metric_bindings (
    metric TEXT PRIMARY KEY NOT NULL CHECK (metric IN ('cards_reviewed', 'study_minutes')),
    statistic_id INTEGER UNIQUE REFERENCES statistics(id) ON DELETE SET NULL
);
CREATE TABLE yomikata_day_totals (
    day TEXT PRIMARY KEY NOT NULL,
    cards_reviewed BIGINT NOT NULL DEFAULT 0 CHECK (cards_reviewed >= 0),
    study_time_seconds BIGINT NOT NULL DEFAULT 0 CHECK (study_time_seconds >= 0)
);
