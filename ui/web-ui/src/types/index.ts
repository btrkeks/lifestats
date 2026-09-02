// Type definitions mirroring backend DTOs

export interface StatisticMetadata {
  id: number;
  name: string;
  category: string;
  statistic_type: StatisticType;
  /**
   * Raw default goal as stored by the backend (same shape as `goal_data` of
   * `GET /statistics/{id}/goals/default`); null when no default goal is set.
   */
  default_goal: Record<string, unknown> | null;
}

// Statistic types matching backend enums
export type StatisticType =
  | 'IntegerAmount'
  | 'Habit'
  | 'PointInTime'
  | 'TimeSpans'
  | 'Summing'
  | 'TimeDifference';

export interface CreateStatisticRequest {
  name: string;
  category: string;
  statistic_type: StatisticType;
  source_statistic_ids?: number[];
}

// ============================================================================
// Goal data (form / request shape)
// ============================================================================

export interface IntegerAmountGoalData {
  target: number;
  is_at_least: boolean;
}

export interface HabitGoalData {
  occurred: boolean;
}

export interface PointInTimeGoalData {
  deadline: string; // "HH:MM" or "HH:MM:SS"
}

export interface TimeSpansGoalData {
  target: { start: string; end: string }[];
  efficiency: number; // 0 to 100
}

export interface SecondsAmountGoalData {
  target: number; // seconds
  is_at_least: boolean;
}

// Discriminated union for all possible goal types
export type GoalData =
  | { type: 'IntegerAmount'; data: IntegerAmountGoalData }
  | { type: 'Habit'; data: HabitGoalData }
  | { type: 'PointInTime'; data: PointInTimeGoalData }
  | { type: 'TimeSpans'; data: TimeSpansGoalData }
  | { type: 'Summing'; data: IntegerAmountGoalData } // Same as IntegerAmount
  | { type: 'TimeDifference'; data: SecondsAmountGoalData };

/** Raw `GET /statistics/{id}/goals/default` body (before parsing) */
export interface GoalResponseRaw {
  goal_data: Record<string, unknown>;
}

/** Parsed goal with type information */
export interface GoalResponse {
  statistic_type: StatisticType;
  goal_data: GoalData;
}

/** Request to set a default goal (all values are strings on the wire) */
export interface SetGoalRequest {
  goal_data: Record<string, string>;
}

/** Request body of `PUT /statistics/{id}/goals/{date}` (serde tag "kind") */
export type SetDailyGoalRequest =
  | { kind: 'goal'; goal_data: Record<string, string> }
  | { kind: 'suppressed' };

/** Raw `GET /statistics/{id}/goals/{date}` body */
export type DailyGoalStateRaw =
  | { kind: 'not_set' }
  | { kind: 'suppressed' }
  | { kind: 'goal'; goal_data: Record<string, unknown> };

/** Parsed daily goal override state for one date */
export type DailyGoalState =
  | { kind: 'not_set' }
  | { kind: 'suppressed' }
  | { kind: 'goal'; goal: GoalResponse };

// ============================================================================
// Daily achievements (`GET /statistics/{id}/achievements`)
// ============================================================================

export type DayOverride = 'none' | 'suppressed' | 'goal';

export interface DailyAchievement {
  date: string; // YYYY-MM-DD
  has_goal: boolean;
  achieved: boolean;
  /** 0-255 (can exceed 100), null when no goal applied that day */
  percentage_achieved: number | null;
  override: DayOverride;
}

export interface DailyAchievementsResponse {
  statistic_id: number;
  start_date: string;
  end_date: string;
  days: DailyAchievement[];
}
