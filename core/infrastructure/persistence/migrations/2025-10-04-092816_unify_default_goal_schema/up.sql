-- Create a new table with the desired single 'goal' column.
CREATE TABLE default_goals_new (
    statistic_id INTEGER NOT NULL PRIMARY KEY,
    goal TEXT NOT NULL,
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

-- Populate the new table by constructing the correct JSON object based on statistic_type.
INSERT INTO default_goals_new (statistic_id, goal)
SELECT
    s.id,
    CASE s.statistic_type
        -- Handle AmountGoal types (IntegerAmount, Summing, TimeDifference)
        WHEN 'IntegerAmount' THEN
            json_object('target_amount', json(dg.target_value), 'is_at_least', IIF(dg.extra_data1 = 'true', json('true'), json('false')))
        WHEN 'Summing' THEN
            json_object('target_amount', json(dg.target_value), 'is_at_least', IIF(dg.extra_data1 = 'true', json('true'), json('false')))
        WHEN 'TimeDifference' THEN
            json_object('target_amount', json(dg.target_value), 'is_at_least', IIF(dg.extra_data1 = 'true', json('true'), json('false')))
        -- Handle HabitGoal type
        WHEN 'Habit' THEN
            json_object('target', IIF(dg.target_value = 'true', json('true'), json('false')))
        -- Handle TimeSpansGoal type
        WHEN 'TimeSpans' THEN
            json_object('target', json_extract(dg.target_value, '$.data'), 'efficiency', json(dg.extra_data1))
        -- Handle FinishedInTimeGoal type
        WHEN 'PointInTime' THEN
            json_object('target', substr(dg.target_value, 2, length(dg.target_value) - 2))
        -- Fallback for any other types
        ELSE
            json_object('target', json(dg.target_value))
    END
FROM
    default_goals dg
JOIN
    statistics s ON dg.statistic_id = s.id;

-- Drop the old table.
DROP TABLE default_goals;

-- Rename the new table to the original name.
ALTER TABLE default_goals_new RENAME TO default_goals;
