-- Create a new table with the desired single 'goal' column.
CREATE TABLE default_goals_new (
    statistic_id INTEGER NOT NULL PRIMARY KEY,
    goal TEXT NOT NULL,
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

-- Populate the new table by constructing a JSON object from the old columns.
-- The CASE statement handles goals that might not have extra_data1.
INSERT INTO default_goals_new (statistic_id, goal)
SELECT
    statistic_id,
    CASE
        WHEN extra_data1 IS NULL THEN
            json_object('target', json(target_value))
        ELSE
            json_object('target', json(target_value), 'extra_data1', extra_data1)
    END
FROM default_goals;

-- Drop the old table.
DROP TABLE default_goals;

-- Rename the new table to the original name.
ALTER TABLE default_goals_new RENAME TO default_goals;
