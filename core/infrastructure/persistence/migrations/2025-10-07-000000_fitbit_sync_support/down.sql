DROP INDEX IF EXISTS idx_logging_data_stat_date;
DROP INDEX IF EXISTS idx_statistics_canonical_tags;
DROP INDEX IF EXISTS idx_integrations_source_name;
DROP TABLE IF EXISTS integrations;
ALTER TABLE statistics DROP COLUMN canonical_tags;
