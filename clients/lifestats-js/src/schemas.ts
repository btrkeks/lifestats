/**
 * Zod validation schemas for LifeStats API requests and responses
 * Provides runtime type checking and validation
 */

import { z } from 'zod';

// ============================================================================
// Statistic Schemas
// ============================================================================

export const statisticTypeSchema = z.enum([
  'IntegerAmount',
  'Habit',
  'PointInTime',
  'TimeSpans',
  'TimedAmounts',
  'Summing',
  'TimeDifference',
]);

export const updatePolicySchema = z.enum(['sum', 'overwrite', 'max', 'min']);

export const statisticMetadataSchema = z.object({
  id: z.number().int().positive(),
  name: z.string().min(1),
  category: z.string().min(1),
  statistic_type: statisticTypeSchema,
});

/** Skip future types in listings while retaining validation for every known type. */
export const statisticListSchema = z
  .array(z.object({ statistic_type: z.string() }).passthrough())
  .transform((statistics) =>
    statistics
      .filter((statistic) => statisticTypeSchema.safeParse(statistic.statistic_type).success)
      .map((statistic) => statisticMetadataSchema.parse(statistic)),
  );

export const createStatisticRequestSchema = z.object({
  name: z.string().min(1),
  category: z.string().min(1),
  statistic_type: statisticTypeSchema,
  source_statistic_ids: z.array(z.number().int().positive()).optional(),
});

export const logValueRequestSchema = z.object({
  value: z.string().min(1),
  update_policy: updatePolicySchema.optional(),
});

const timeFormatRegex = /^\d{2}:\d{2}(:\d{2})?$/;

export const logTimeFrameRequestSchema = z.object({
  start_time: z.string().regex(timeFormatRegex, 'Time must be in format HH:MM or HH:MM:SS'),
  end_time: z.string().regex(timeFormatRegex, 'Time must be in format HH:MM or HH:MM:SS'),
  update_policy: updatePolicySchema.optional(),
});

// ============================================================================
// Goal Schemas
// ============================================================================

export const goalDataSchema = z.record(z.string(), z.string());

export const setDefaultGoalRequestSchema = z.object({
  goal_data: goalDataSchema,
});

export const setDailyGoalRequestSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('goal'),
    goal_data: goalDataSchema,
  }),
  z.object({
    kind: z.literal('suppressed'),
  }),
]);

export const goalResponseSchema = z.object({
  goal_data: z.unknown(), // JSON value - structure varies by statistic type
});

// ============================================================================
// Goal Data Builders (Type-Safe Constructors)
// ============================================================================

/**
 * Create goal data for IntegerAmount statistics
 */
export function createIntegerAmountGoal(target: number, isAtLeast: boolean = true): Record<string, string> {
  return {
    target: String(target),
    is_at_least: String(isAtLeast),
  };
}

/**
 * Create goal data for Habit statistics
 */
export function createHabitGoal(occurred: boolean): Record<string, string> {
  return {
    occurred: String(occurred),
  };
}

/**
 * Create goal data for PointInTime statistics
 */
export function createPointInTimeGoal(deadline: string): Record<string, string> {
  // Validate time format
  if (!timeFormatRegex.test(deadline)) {
    throw new Error('Deadline must be in format HH:MM or HH:MM:SS');
  }
  return {
    deadline,
  };
}

/**
 * Create goal data for TimeSpans statistics
 */
export function createTimeSpansGoal(
  targetFrames: Array<{ start: string; end: string }>,
  efficiency?: number,
): Record<string, string> {
  const goalData: Record<string, string> = {
    target: JSON.stringify(targetFrames),
  };
  if (efficiency !== undefined) {
    goalData.efficiency = String(efficiency);
  }
  return goalData;
}

/**
 * Create goal data for Summing/TimeDifference statistics
 */
export function createDurationGoal(targetSeconds: number, isAtLeast: boolean = true): Record<string, string> {
  return {
    target: String(targetSeconds),
    is_at_least: String(isAtLeast),
  };
}

// ============================================================================
// Date Helpers
// ============================================================================

/**
 * Format a Date object as YYYY-MM-DD
 */
export function formatDate(date: Date): string {
  return date.toISOString().split('T')[0];
}

/**
 * Format a Date object as HH:MM:SS
 */
export function formatTime(date: Date): string {
  return date.toTimeString().split(' ')[0];
}

/**
 * Validate date string is in YYYY-MM-DD format
 */
export function validateDateString(dateStr: string): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(dateStr);
}
