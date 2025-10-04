-- This is complex and likely not needed for production, but is good practice.
-- For this refactor, we can accept a simplified down migration.
CREATE TABLE goals_old (
    statistic_id INTEGER NOT NULL,
    date TEXT NOT NULL,
    target_value TEXT,
    extra_data1 TEXT,
    PRIMARY KEY (statistic_id, date),
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);
-- Data loss is acceptable on downgrade for this specific refactor.
DROP TABLE goals;
ALTER TABLE goals_old RENAME TO goals;
