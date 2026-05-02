CREATE TABLE progress_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    unit_kind TEXT NOT NULL,
    unit_label TEXT,
    target_value BIGINT NOT NULL,
    tags TEXT NOT NULL DEFAULT '{}',
    canonical_tags TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK (length(trim(name)) > 0),
    CHECK (length(trim(category)) > 0),
    CHECK (target_value > 0)
);

CREATE INDEX idx_progress_items_category
    ON progress_items(category);

CREATE INDEX idx_progress_items_canonical_tags
    ON progress_items(canonical_tags);

CREATE TABLE progress_checkpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    progress_item_id INTEGER NOT NULL,
    recorded_at TEXT NOT NULL,
    effective_date TEXT NOT NULL,
    position_value BIGINT NOT NULL,
    source TEXT,
    source_event_id TEXT,
    note TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY(progress_item_id) REFERENCES progress_items(id) ON DELETE CASCADE,
    CHECK (position_value >= 0)
);

CREATE INDEX idx_progress_checkpoints_item_recorded_at
    ON progress_checkpoints(progress_item_id, recorded_at, id);

CREATE INDEX idx_progress_checkpoints_item_effective_date
    ON progress_checkpoints(progress_item_id, effective_date);

CREATE UNIQUE INDEX idx_progress_checkpoints_source_event
    ON progress_checkpoints(progress_item_id, source, source_event_id)
    WHERE source IS NOT NULL AND source_event_id IS NOT NULL;
