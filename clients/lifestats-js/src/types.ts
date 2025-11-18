/**
 * Core type definitions for the LifeStats API
 * These types mirror the backend DTOs defined in core/infrastructure/web_api/src/dto/
 */

// ============================================================================
// Statistic Types
// ============================================================================

export type StatisticType =
  | 'IntegerAmount'
  | 'Habit'
  | 'PointInTime'
  | 'TimeSpans'
  | 'Summing'
  | 'TimeDifference';

export type UpdatePolicy = 'sum' | 'overwrite' | 'max' | 'min';

/**
 * Metadata for a statistic (response from API)
 */
export interface StatisticMetadata {
  id: number;
  name: string;
  category: string;
  statistic_type: StatisticType;
}

/**
 * Request payload for creating a new statistic
 */
export interface CreateStatisticRequest {
  name: string;
  category: string;
  statistic_type: StatisticType;
  /** Required for Summing and TimeDifference statistics */
  source_statistic_ids?: number[];
}

/**
 * Request payload for logging a value
 */
export interface LogValueRequest {
  value: string;
  update_policy?: UpdatePolicy;
}

/**
 * Request payload for logging a time frame (TimeSpans only)
 */
export interface LogTimeFrameRequest {
  start_time: string; // Format: HH:MM or HH:MM:SS
  end_time: string; // Format: HH:MM or HH:MM:SS
  update_policy?: UpdatePolicy;
}

// ============================================================================
// Goal Types
// ============================================================================

/**
 * Goal data is a HashMap<String, String> on the backend
 * Different statistic types expect different fields:
 *
 * IntegerAmount: { target: "100", is_at_least: "true" }
 * Habit: { occurred: "true" }
 * PointInTime: { deadline: "23:00" }
 * TimeSpans: { target: "[{\"start\":\"09:00:00\",\"end\":\"17:00:00\"}]", efficiency: "80" }
 * Summing/TimeDifference: { target: "3600", is_at_least: "true" }
 */
export type GoalData = Record<string, string>;

/**
 * Request payload for setting a default goal
 */
export interface SetDefaultGoalRequest {
  goal_data: GoalData;
}

/**
 * Request payload for setting a daily goal
 * Can be either a goal or a suppression (removes default goal for that day)
 */
export type SetDailyGoalRequest =
  | { kind: 'goal'; goal_data: GoalData }
  | { kind: 'suppressed' };

/**
 * Response containing goal data
 */
export interface GoalResponse {
  goal_data: unknown; // JSON value - structure depends on statistic type
}

// ============================================================================
// Date/Time Types
// ============================================================================

/**
 * Date string in ISO format: YYYY-MM-DD
 */
export type DateString = string;

/**
 * Time string in format: HH:MM or HH:MM:SS
 */
export type TimeString = string;

// ============================================================================
// Error Types
// ============================================================================

export class LifeStatsError extends Error {
  constructor(
    message: string,
    public statusCode?: number,
    public response?: unknown,
  ) {
    super(message);
    this.name = 'LifeStatsError';
  }
}
