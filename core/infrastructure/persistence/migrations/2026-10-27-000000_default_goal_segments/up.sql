-- The default goal over time: each row applies from valid_since until the next
-- row of the same statistic starts. goal NULL means "no default goal".
-- Statistics without rows still use default_goals + statistics.default_goal_valid_since.
CREATE TABLE default_goal_segments (
    statistic_id INTEGER NOT NULL REFERENCES statistics(id) ON DELETE CASCADE,
    valid_since TEXT NOT NULL,
    goal TEXT,
    PRIMARY KEY (statistic_id, valid_since)
);
