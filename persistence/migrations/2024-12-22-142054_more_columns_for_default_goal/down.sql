-- Create a table with the original schema
CREATE TABLE default_goals_old (
                                   statistic_id INTEGER NOT NULL PRIMARY KEY,
                                   goal TEXT,
                                   FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

-- Drop the new format table
DROP TABLE default_goals;

-- Rename the old format table back
ALTER TABLE default_goals_old RENAME TO default_goals;