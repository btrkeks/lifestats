-- Remove UNIQUE constraint from canonical_tags column
-- by dropping the unique index and recreating as a non-unique index
DROP INDEX IF EXISTS idx_statistics_canonical_tags;
CREATE INDEX IF NOT EXISTS idx_statistics_canonical_tags ON statistics(canonical_tags);
