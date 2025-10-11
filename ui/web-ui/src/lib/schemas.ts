import { z } from 'zod';
import type { StatisticType } from '@/types';

// ============================================================================
// Zod Schemas for Goal Data Validation
// ============================================================================

// Schema for IntegerAmount and Summing goals
export const integerAmountGoalSchema = z.object({
  target: z.coerce.number().int(),
  is_at_least: z
    .union([z.boolean(), z.string()])
    .transform((val) => (typeof val === 'string' ? val === 'true' : val)),
});

// Schema for Habit goals
export const habitGoalSchema = z.object({
  occurred: z
    .union([z.boolean(), z.string()])
    .transform((val) => (typeof val === 'string' ? val === 'true' : val)),
});

// Schema for PointInTime goals
export const pointInTimeGoalSchema = z.object({
  deadline: z.string().regex(/^\d{2}:\d{2}(:\d{2})?$/, 'Must be in HH:MM or HH:MM:SS format'),
});

// Schema for TimeSpans goals
export const timeSpansGoalSchema = z.object({
  target: z
    .array(
      z.object({
        start: z
          .string()
          .regex(/^\d{2}:\d{2}(:\d{2})?$/, 'Start time must be HH:MM or HH:MM:SS'),
        end: z
          .string()
          .regex(/^\d{2}:\d{2}(:\d{2})?$/, 'End time must be HH:MM or HH:MM:SS'),
      })
    )
    .min(1, 'At least one time frame is required.'),
  efficiency: z
    .coerce
    .number()
    .min(0, 'Efficiency cannot be negative.')
    .max(100, 'Efficiency cannot exceed 100.'),
});

// Schema for TimeDifference goals
export const secondsAmountGoalSchema = z.object({
  target: z.coerce.number(),
  is_at_least: z
    .union([z.boolean(), z.string()])
    .transform((val) => (typeof val === 'string' ? val === 'true' : val)),
});

// ============================================================================
// Goal Data Parser
// ============================================================================

/**
 * Parses raw goal data from the backend into strongly-typed frontend format.
 * This is the "type safety boundary" - data that passes through this function
 * is guaranteed to match our discriminated union types.
 */
export function parseGoalData(statisticType: StatisticType, rawData: Record<string, unknown>) {
  switch (statisticType) {
    case 'IntegerAmount':
      return {
        type: 'IntegerAmount' as const,
        data: integerAmountGoalSchema.parse(rawData),
      };
    case 'Habit':
      return {
        type: 'Habit' as const,
        data: habitGoalSchema.parse(rawData),
      };
    case 'PointInTime':
      return {
        type: 'PointInTime' as const,
        data: pointInTimeGoalSchema.parse(rawData),
      };
    case 'TimeSpans':
      return {
        type: 'TimeSpans' as const,
        data: timeSpansGoalSchema.parse(rawData),
      };
    case 'Summing':
      return {
        type: 'Summing' as const,
        data: integerAmountGoalSchema.parse(rawData),
      };
    case 'TimeDifference':
      return {
        type: 'TimeDifference' as const,
        data: secondsAmountGoalSchema.parse(rawData),
      };
    default:
      throw new Error(`Unknown statistic type: ${statisticType}`);
  }
}

// ============================================================================
// Goal Data Serializer
// ============================================================================

/**
 * Converts strongly-typed goal data back to the backend's expected format
 * (all values as strings in a Record<string, string>).
 */
export function serializeGoalData(goalData: Record<string, unknown>): Record<string, string> {
  const result: Record<string, string> = {};

  Object.entries(goalData).forEach(([key, value]) => {
    if (value === undefined || value === null) {
      return;
    }

    if (typeof value === 'string') {
      result[key] = value;
      return;
    }

    if (typeof value === 'number' || typeof value === 'boolean') {
      result[key] = String(value);
      return;
    }

    result[key] = JSON.stringify(value);
  });

  return result;
}
