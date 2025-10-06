-- Add tags column to statistics table
-- Default to empty JSON object for backward compatibility
ALTER TABLE statistics ADD COLUMN tags TEXT NOT NULL DEFAULT '{}';
