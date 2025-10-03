ALTER TABLE logging_data ADD COLUMN goal TEXT;

-- Migrate data back if needed
UPDATE logging_data
SET goal = (
    SELECT target_value
    FROM goals
    WHERE goals.statistic_id = logging_data.statistic_id
      AND goals.date = logging_data.date
);

DROP TABLE goals;
DROP TABLE default_goals;
DROP TABLE plots;
DROP TABLE statistics;