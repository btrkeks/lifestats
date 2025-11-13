-- Recreate the plots table for rollback
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
