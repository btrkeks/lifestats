-- First create the statistics table
CREATE TABLE statistics
(
    id             INTEGER NOT NULL PRIMARY KEY,
    name           TEXT    NOT NULL,
    category       TEXT    NOT NULL,
    statistic_type TEXT    NOT NULL
);

CREATE TABLE plots
(
    id           INTEGER NOT NULL PRIMARY KEY,
    statistic_id INTEGER NOT NULL,
    title        TEXT    NOT NULL,
    width        INTEGER NOT NULL CHECK (width > 0),
    height       INTEGER NOT NULL CHECK (height > 0),
    plot_type    TEXT    NOT NULL,
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

CREATE TABLE default_goals
(
    statistic_id INTEGER NOT NULL PRIMARY KEY,
    goal         TEXT,
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

-- Create the new goals table
CREATE TABLE goals
(
    statistic_id INTEGER NOT NULL,
    date         TEXT    NOT NULL,
    target_value TEXT, -- JSON serialized target value
    extra_data1  TEXT, -- Additional goal parameters
    PRIMARY KEY (statistic_id, date),
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

-- Migrate existing goal data from logging_data table
INSERT INTO goals (statistic_id, date, target_value, extra_data1)
SELECT statistic_id, date, goal, -- Store the entire goal JSON as target_value for now
    NULL                         -- No extra data in current schema
FROM logging_data
WHERE goal IS NOT NULL;

ALTER TABLE logging_data DROP COLUMN goal;
