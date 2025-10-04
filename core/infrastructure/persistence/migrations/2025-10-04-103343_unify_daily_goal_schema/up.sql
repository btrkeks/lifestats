-- Create a new table with the desired single 'goal' column.
CREATE TABLE goals_new (
    statistic_id INTEGER NOT NULL,
    date TEXT NOT NULL,
    goal TEXT,
    PRIMARY KEY (statistic_id, date),
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

-- Populate the new table by constructing the correct JSON object based on statistic_type.
INSERT INTO goals_new (statistic_id, date, goal)
SELECT
    s.id,
    g.date,
    CASE
        WHEN g.target_value IS NULL THEN NULL
        ELSE
            CASE s.statistic_type
                -- Handle AmountGoal types (IntegerAmount, Summing, TimeDifference)
                WHEN 'IntegerAmount' THEN
                    json_object('target_amount', json(g.target_value), 'is_at_least', IIF(g.extra_data1 = 'true', json('true'), json('false')))
                WHEN 'Summing' THEN
                    json_object('target_amount', json(g.target_value), 'is_at_least', IIF(g.extra_data1 = 'true', json('true'), json('false')))
                WHEN 'TimeDifference' THEN
                    json_object('target_amount', json(g.target_value), 'is_at_least', IIF(g.extra_data1 = 'true', json('true'), json('false')))
                -- Handle HabitGoal type
                WHEN 'Habit' THEN
                    json_object('target', IIF(g.target_value = 'true', json('true'), json('false')))
                -- Handle TimeSpansGoal type
                WHEN 'TimeSpans' THEN
                    json_object('target', json_extract(g.target_value, '$.data'), 'efficiency', json(g.extra_data1))
                -- Handle FinishedInTimeGoal type
                WHEN 'PointInTime' THEN
                    json_object('target', substr(g.target_value, 2, length(g.target_value) - 2))
                -- Fallback for any other types
                ELSE
                    json_object('target', json(g.target_value))
            END
    END
FROM
    goals g
JOIN
    statistics s ON g.statistic_id = s.id;

-- Drop the old table.
DROP TABLE goals;

-- Rename the new table to the original name.
ALTER TABLE goals_new RENAME TO goals;
