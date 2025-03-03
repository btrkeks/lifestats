-- First, create a new table with the desired schema
CREATE TABLE default_goals_new (
                                   statistic_id INTEGER NOT NULL PRIMARY KEY,
                                   target_value TEXT NOT NULL,
                                   extra_data1 TEXT,
                                   FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

-- Drop the old table
DROP TABLE default_goals;

-- Rename the new table to the original name
ALTER TABLE default_goals_new RENAME TO default_goals;