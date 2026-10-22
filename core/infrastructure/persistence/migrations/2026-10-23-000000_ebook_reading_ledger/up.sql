CREATE TABLE ebook_server_identity (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    server_id TEXT NOT NULL CHECK (length(server_id) = 36)
);
CREATE TABLE ebook_books (
    book_key TEXT PRIMARY KEY NOT NULL CHECK (length(book_key) = 76 AND substr(book_key,1,12) = 'ttu-text-v1:'),
    length BIGINT NOT NULL CHECK (length > 0 AND length <= 9007199254740992),
    progress_item_id INTEGER REFERENCES progress_items(id) ON DELETE SET NULL,
    legacy_spans TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_ebook_books_progress_item ON ebook_books(progress_item_id) WHERE progress_item_id IS NOT NULL;
CREATE TABLE ebook_day_spans (
    producer_id TEXT NOT NULL CHECK (length(producer_id) = 36),
    book_key TEXT NOT NULL REFERENCES ebook_books(book_key),
    day TEXT NOT NULL CHECK (length(day) = 10),
    spans TEXT NOT NULL,
    last_position BIGINT NOT NULL CHECK (last_position >= 0),
    observed_at TEXT NOT NULL,
    PRIMARY KEY (producer_id, book_key, day)
);
CREATE INDEX idx_ebook_day_spans_book_day ON ebook_day_spans(book_key, day);
CREATE TABLE ebook_day_credits (
    book_key TEXT NOT NULL REFERENCES ebook_books(book_key),
    day TEXT NOT NULL CHECK (length(day) = 10),
    characters BIGINT NOT NULL CHECK (characters >= 0),
    PRIMARY KEY (book_key, day)
);
CREATE TABLE ebook_metric_bindings (
    metric TEXT PRIMARY KEY NOT NULL CHECK (metric = 'characters_read'),
    statistic_id INTEGER UNIQUE REFERENCES statistics(id) ON DELETE SET NULL
);
CREATE TABLE ebook_legacy_bindings (
    reader_data_id INTEGER PRIMARY KEY NOT NULL CHECK (reader_data_id > 0),
    book_key TEXT NOT NULL UNIQUE REFERENCES ebook_books(book_key)
);
