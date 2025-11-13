-- Restore UNIQUE constraint on canonical_tags column
-- by dropping the non-unique index and recreating as a unique index
DROP INDEX IF EXISTS idx_statistics_canonical_tags;
CREATE UNIQUE INDEX IF NOT EXISTS idx_statistics_canonical_tags ON statistics(canonical_tags);
