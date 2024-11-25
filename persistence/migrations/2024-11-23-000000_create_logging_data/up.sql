CREATE TABLE IF NOT EXISTS logging_data (
     statistic_id INTEGER NOT NULL,
     date TEXT NOT NULL,
     value TEXT,
     goal TEXT,
     PRIMARY KEY (statistic_id, date)
);