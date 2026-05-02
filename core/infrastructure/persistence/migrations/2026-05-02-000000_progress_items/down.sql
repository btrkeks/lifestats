DROP INDEX IF EXISTS idx_progress_checkpoints_source_event;
DROP INDEX IF EXISTS idx_progress_checkpoints_item_effective_date;
DROP INDEX IF EXISTS idx_progress_checkpoints_item_recorded_at;
DROP TABLE IF EXISTS progress_checkpoints;

DROP INDEX IF EXISTS idx_progress_items_canonical_tags;
DROP INDEX IF EXISTS idx_progress_items_category;
DROP TABLE IF EXISTS progress_items;
