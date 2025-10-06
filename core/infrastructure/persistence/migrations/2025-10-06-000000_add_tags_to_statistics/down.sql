-- Remove tags column from statistics table
-- SQLite doesn't support DROP COLUMN directly on older versions
-- We create a new table without the column and copy the data

CREATE TABLE statistics_new (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    statistic_type TEXT NOT NULL,
    default_goal_valid_since TEXT NOT NULL
);

INSERT INTO statistics_new (id, name, category, statistic_type, default_goal_valid_since)
SELECT id, name, category, statistic_type, default_goal_valid_since
FROM statistics;

DROP TABLE statistics;

ALTER TABLE statistics_new RENAME TO statistics;
