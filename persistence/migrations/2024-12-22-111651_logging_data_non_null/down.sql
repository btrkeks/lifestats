-- Create the original table structure
CREATE TABLE logging_data_old (
                                  statistic_id INTEGER,
                                  date TEXT,
                                  value TEXT,
                                  PRIMARY KEY (statistic_id, date)
);

-- Copy data from the new table
INSERT INTO logging_data_old
SELECT * FROM logging_data;

-- Drop the new table
DROP TABLE logging_data;

-- Rename the old table back
ALTER TABLE logging_data_old RENAME TO logging_data;