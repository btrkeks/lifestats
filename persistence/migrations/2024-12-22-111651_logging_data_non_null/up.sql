-- First, create a new table with the correct schema
CREATE TABLE logging_data_new (
                                  statistic_id INTEGER NOT NULL,
                                  date TEXT NOT NULL,
                                  value TEXT NOT NULL,
                                  PRIMARY KEY (statistic_id, date)
);

-- Copy only valid data from the old table
INSERT INTO logging_data_new (statistic_id, date, value)
SELECT statistic_id, date, value
FROM logging_data
WHERE statistic_id IS NOT NULL
  AND date IS NOT NULL
  AND value IS NOT NULL;

-- Drop the old table
DROP TABLE logging_data;

-- Rename the new table to the original name
ALTER TABLE logging_data_new RENAME TO logging_data;