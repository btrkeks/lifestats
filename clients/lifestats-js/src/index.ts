/**
 * @lifestats/client
 * TypeScript client library for the LifeStats API
 */

// Export the main client
export { LifeStatsClient } from './client';
export type { ClientOptions } from './client';

// Export all types
export type {
  StatisticType,
  UpdatePolicy,
  StatisticMetadata,
  CreateStatisticRequest,
  LogValueRequest,
  LogTimeFrameRequest,
  GoalData,
  SetDefaultGoalRequest,
  SetDailyGoalRequest,
  GoalResponse,
  DateString,
  TimeString,
} from './types';
export { LifeStatsError } from './types';

// Export schemas and helpers
export {
  statisticTypeSchema,
  updatePolicySchema,
  statisticMetadataSchema,
  createStatisticRequestSchema,
  logValueRequestSchema,
  logTimeFrameRequestSchema,
  goalDataSchema,
  setDefaultGoalRequestSchema,
  setDailyGoalRequestSchema,
  goalResponseSchema,
  createIntegerAmountGoal,
  createHabitGoal,
  createPointInTimeGoal,
  createTimeSpansGoal,
  createDurationGoal,
  formatDate,
  formatTime,
  validateDateString,
} from './schemas';
