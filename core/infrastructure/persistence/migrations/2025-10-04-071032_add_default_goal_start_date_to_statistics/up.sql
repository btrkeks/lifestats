-- Add the new column to the statistics table
ALTER TABLE statistics ADD COLUMN default_goal_valid_since TEXT NOT NULL DEFAULT '1970-01-01';
