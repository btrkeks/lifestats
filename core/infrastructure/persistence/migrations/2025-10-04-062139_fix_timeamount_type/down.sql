-- This file should undo anything in `up.sql`
UPDATE statistics
SET statistic_type = 'TimeAmount'
WHERE statistic_type = 'IntegerAmount';
