// Type definitions mirroring backend DTOs

export interface StatisticMetadata {
  id: number;
  name: string;
  category: string;
  statistic_type: StatisticType;
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
// Goal Data Types - Strongly Typed (for frontend use)
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

// ============================================================================
// API Request/Response Types
// ============================================================================

// Raw response from backend (before parsing)
export interface GoalResponseRaw {
  goal_data: Record<string, unknown>; // JSON value from backend
}

// Parsed response with type information
export interface GoalResponse {
  statistic_type: StatisticType;
  goal_data: GoalData;
}

// Request to set a default goal (must match backend string format)
export interface SetGoalRequest {
  goal_data: Record<string, string>;
}

// Discriminated union for setting daily goals
// Matches backend SetDailyGoalRequest enum with serde tag "kind"
export type SetDailyGoalRequest =
  | { kind: 'goal'; goal_data: Record<string, string> }
  | { kind: 'suppressed' };

// Response that may indicate suppressed goal
export interface GoalResponseWithSuppress extends GoalResponse {
  suppressed?: true; // Present and true if goal is suppressed
}
