CREATE TABLE ebook_legacy_bindings (
    reader_data_id INTEGER PRIMARY KEY NOT NULL CHECK (reader_data_id > 0),
    book_key TEXT NOT NULL UNIQUE REFERENCES ebook_books(book_key)
);
