-- Fix schema drift: Update old 'TimeAmount' type to 'IntegerAmount'
UPDATE statistics
SET statistic_type = 'IntegerAmount'
WHERE statistic_type = 'TimeAmount';
