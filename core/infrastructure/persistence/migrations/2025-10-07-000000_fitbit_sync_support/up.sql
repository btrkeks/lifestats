-- Ensure the integrations table exists with Fitbit token storage columns
CREATE TABLE IF NOT EXISTS integrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_name TEXT NOT NULL UNIQUE,
    access_token TEXT,
    refresh_token TEXT,
    expires_at_epoch_seconds INTEGER,
    scopes TEXT,
    last_sync_at TEXT,
    canonical_tags TEXT
);

-- Add helpful lookup index for integrations by source name
CREATE INDEX IF NOT EXISTS idx_integrations_source_name ON integrations(source_name);

-- Canonical tags support for statistics (enables TagSet fingerprint lookups)
ALTER TABLE statistics ADD COLUMN canonical_tags TEXT;

-- Backfill canonical tag fingerprints from existing JSON tag payloads
UPDATE statistics
SET canonical_tags = (
    SELECT GROUP_CONCAT(pair, ';')
    FROM (
        SELECT printf('%s=%s', lower(trim(json_each.key)), trim(json_each.value)) AS pair
        FROM json_each(statistics.tags)
        ORDER BY lower(trim(json_each.key))
    )
)
WHERE canonical_tags IS NULL;

-- Enforce uniqueness on canonical tag fingerprints
CREATE UNIQUE INDEX IF NOT EXISTS idx_statistics_canonical_tags ON statistics(canonical_tags);

-- Guarantee logging data uniqueness for UPSERT semantics
CREATE UNIQUE INDEX IF NOT EXISTS idx_logging_data_stat_date ON logging_data(statistic_id, date);
