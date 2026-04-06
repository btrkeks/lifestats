CREATE TABLE IF NOT EXISTS ebook_sync_events (
    event_id TEXT NOT NULL PRIMARY KEY,
    source_name TEXT NOT NULL,
    metric TEXT NOT NULL,
    statistic_id INTEGER NOT NULL,
    date TEXT NOT NULL,
    delta INTEGER NOT NULL,
    book_id TEXT,
    book_title TEXT NOT NULL,
    confirmed_position_after INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (statistic_id) REFERENCES statistics (id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ebook_sync_events_stat_date
    ON ebook_sync_events(statistic_id, date);
