-- Recreate table without the new column to support SQLite's limitations
CREATE TABLE statistics_new (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    statistic_type TEXT NOT NULL
);

INSERT INTO statistics_new (id, name, category, statistic_type)
SELECT id, name, category, statistic_type FROM statistics;

DROP TABLE statistics;

ALTER TABLE statistics_new RENAME TO statistics;
