-- This is complex and likely not needed for production, but is good practice.
-- For this refactor, we can accept a simplified down migration.
CREATE TABLE default_goals_old (
    statistic_id INTEGER NOT NULL PRIMARY KEY,
    target_value TEXT NOT NULL,
    extra_data1 TEXT,
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);
-- Data loss is acceptable on downgrade for this specific refactor.
DROP TABLE default_goals;
ALTER TABLE default_goals_old RENAME TO default_goals;
