import { z } from 'zod';
import { STATISTIC_TYPES } from './statisticTypes';
import type { FieldValues } from 'react-hook-form';
import type {
  DailyGoalState,
  DailyGoalStateRaw,
  GoalData,
  SetDailyGoalRequest,
  StatisticMetadata,
  StatisticType,
} from '@/types';

const timeString = (label: string) =>
  z.string().regex(/^\d{2}:\d{2}(:\d{2})?$/, `${label} must be HH:MM or HH:MM:SS`);

const booleanish = z
  .union([z.boolean(), z.string()])
  .transform((val) => (typeof val === 'string' ? val === 'true' : val));

// ============================================================================
// Form schemas (validate what the user types; shape matches the PUT request)
// ============================================================================

// IntegerAmount and Summing goals
export const integerAmountGoalSchema = z.object({
  target: z.coerce.number().int(),
  is_at_least: booleanish,
});

export const timedAmountsGoalSchema = z.object({
  target: z.coerce.number().int().min(0).max(Number.MAX_SAFE_INTEGER),
  is_at_least: booleanish,
});

// Habit goals
export const habitGoalSchema = z.object({
  occurred: booleanish,
});

// PointInTime goals
export const pointInTimeGoalSchema = z.object({
  deadline: timeString('Deadline'),
});

const timeFrameSchema = z.object({
  start: timeString('Start time'),
  end: timeString('End time'),
});

// TimeSpans goals
export const timeSpansGoalSchema = z.object({
  target: z.array(timeFrameSchema).min(1, 'At least one time frame is required.'),
  efficiency: z.coerce
    .number()
    .min(0, 'Efficiency cannot be negative.')
    .max(100, 'Efficiency cannot exceed 100.'),
});

// TimeDifference goals
export const secondsAmountGoalSchema = z.object({
  target: z.coerce.number(),
  is_at_least: booleanish,
});

/** The form validation schema for a statistic type. */
export function goalFormSchemaFor(
  statisticType: StatisticType
): z.ZodType<FieldValues, FieldValues> {
  switch (statisticType) {
    case 'IntegerAmount':
    case 'Summing':
      return integerAmountGoalSchema;
    case 'TimedAmounts':
      return timedAmountsGoalSchema;
    case 'Habit':
      return habitGoalSchema;
    case 'PointInTime':
      return pointInTimeGoalSchema;
    case 'TimeSpans':
      return timeSpansGoalSchema;
    case 'TimeDifference':
      return secondsAmountGoalSchema;
  }
}

/** Sensible starting values for a new goal of the given type. */
export function goalFormDefaultsFor(statisticType: StatisticType): FieldValues {
  switch (statisticType) {
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference':
      return { target: 0, is_at_least: true };
    case 'TimedAmounts':
      return { target: 0, is_at_least: false };
    case 'Habit':
      return { occurred: true };
    case 'PointInTime':
      return { deadline: '22:00' };
    case 'TimeSpans':
      return { target: [{ start: '09:00', end: '17:00' }], efficiency: 80 };
  }
}

/** The goal the form currently describes, or null while it doesn't validate. */
export function formGoal(statisticType: StatisticType, values: unknown): GoalData | null {
  switch (statisticType) {
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference': {
      const parsed = integerAmountGoalSchema.safeParse(values);
      return parsed.success ? { type: statisticType, data: parsed.data } : null;
    }
    case 'TimedAmounts': {
      const parsed = timedAmountsGoalSchema.safeParse(values);
      return parsed.success ? { type: 'TimedAmounts', data: parsed.data } : null;
    }
    case 'Habit': {
      const parsed = habitGoalSchema.safeParse(values);
      return parsed.success ? { type: 'Habit', data: parsed.data } : null;
    }
    case 'PointInTime': {
      const parsed = pointInTimeGoalSchema.safeParse(values);
      return parsed.success ? { type: 'PointInTime', data: parsed.data } : null;
    }
    case 'TimeSpans': {
      const parsed = timeSpansGoalSchema.safeParse(values);
      return parsed.success ? { type: 'TimeSpans', data: parsed.data } : null;
    }
  }
}

// ============================================================================
// Response schemas (what the backend stores: domain field names)
// ============================================================================

// AmountGoal → {"target_amount": 10, "is_at_least": true}
const amountGoalResponseSchema = z
  .object({ target_amount: z.number(), is_at_least: z.boolean() })
  .transform((raw) => ({ target: raw.target_amount, is_at_least: raw.is_at_least }));

const timedAmountsGoalResponseSchema = z
  .object({
    target_amount: z.number().int().min(0).max(Number.MAX_SAFE_INTEGER),
    is_at_least: z.boolean(),
  })
  .transform((raw) => ({ target: raw.target_amount, is_at_least: raw.is_at_least }));

// HabitGoal → {"target": true}
const habitGoalResponseSchema = z
  .object({ target: z.boolean() })
  .transform((raw) => ({ occurred: raw.target }));

// FinishedInTimeGoal → {"target": "22:00:00"}
const pointInTimeGoalResponseSchema = z
  .object({ target: timeString('Deadline') })
  .transform((raw) => ({ deadline: raw.target }));

// TimeSpansGoal → {"target": [{start, end}], "efficiency": 90}
// (`target` is tolerated as a JSON string for older rows)
const timeSpansGoalResponseSchema = z.object({
  target: z.preprocess(
    (value) => (typeof value === 'string' ? JSON.parse(value) : value),
    z.array(timeFrameSchema)
  ),
  efficiency: z.number(),
});

// ============================================================================
// Goal Data Parser
// ============================================================================

/**
 * Parses raw goal data from the backend into the strongly-typed frontend format.
 * This is the type-safety boundary: data that passes through here is guaranteed
 * to match the discriminated union in `GoalData`.
 */
export function parseGoalData(
  statisticType: StatisticType,
  rawData: Record<string, unknown>
): GoalData {
  switch (statisticType) {
    case 'IntegerAmount':
      return { type: 'IntegerAmount', data: amountGoalResponseSchema.parse(rawData) };
    case 'TimedAmounts':
      return { type: 'TimedAmounts', data: timedAmountsGoalResponseSchema.parse(rawData) };
    case 'Summing':
      return { type: 'Summing', data: amountGoalResponseSchema.parse(rawData) };
    case 'TimeDifference':
      return { type: 'TimeDifference', data: amountGoalResponseSchema.parse(rawData) };
    case 'Habit':
      return { type: 'Habit', data: habitGoalResponseSchema.parse(rawData) };
    case 'PointInTime':
      return { type: 'PointInTime', data: pointInTimeGoalResponseSchema.parse(rawData) };
    case 'TimeSpans':
      return { type: 'TimeSpans', data: timeSpansGoalResponseSchema.parse(rawData) };
  }
}

/** Parses the `default_goal` carried by statistic metadata; null when unset or unreadable. */
export function parseStatisticDefaultGoal(statistic: StatisticMetadata): GoalData | null {
  if (!statistic.default_goal) {
    return null;
  }
  try {
    return parseGoalData(statistic.statistic_type, statistic.default_goal);
  } catch {
    return null;
  }
}

/** Parses the tri-state response of `GET /statistics/{id}/goals/{date}`. */
export function parseDailyGoalState(
  statisticType: StatisticType,
  raw: DailyGoalStateRaw
): DailyGoalState {
  if (raw.kind === 'goal') {
    return {
      kind: 'goal',
      goal: {
        statistic_type: statisticType,
        goal_data: parseGoalData(statisticType, raw.goal_data),
      },
    };
  }
  return raw;
}

// ============================================================================
// Goal Data Serializer
// ============================================================================

/**
 * Converts form values to the backend's request format: every value a string,
 * objects and arrays JSON-encoded (e.g. the TimeSpans frame list).
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

// ============================================================================
// Daily Goal Request Builders
// ============================================================================

/** Request for a concrete daily goal override. */
export function createGoalRequest(goalData: Record<string, unknown>): SetDailyGoalRequest {
  return { kind: 'goal', goal_data: serializeGoalData(goalData) };
}

/** Request that suppresses the goal on a date (no goal applies, default ignored). */
export function createSuppressedRequest(): SetDailyGoalRequest {
  return { kind: 'suppressed' };
}

/** Known types remain validated; future statistic types are omitted from lists. */
const statisticTypeSchema = z.custom<StatisticType>(
  (value) => typeof value === 'string' && Object.prototype.hasOwnProperty.call(STATISTIC_TYPES, value)
);

const statisticMetadataSchema = z.object({
  id: z.number().int().positive(),
  name: z.string(),
  category: z.string(),
  statistic_type: statisticTypeSchema,
  default_goal: z.record(z.string(), z.unknown()).nullable(),
});

export const statisticListSchema = z
  .array(z.object({ statistic_type: z.string() }).passthrough())
  .transform((statistics): StatisticMetadata[] =>
    statistics
      .filter((statistic) => statisticTypeSchema.safeParse(statistic.statistic_type).success)
      .map((statistic) => statisticMetadataSchema.parse(statistic))
  );
