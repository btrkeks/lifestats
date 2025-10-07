// Type definitions mirroring backend DTOs

export interface StatisticMetadata {
  id: number;
  name: string;
  category: string;
}

export interface Goal {
  goal_data: Record<string, string>;
}

export interface GoalResponse {
  goal_data: Record<string, unknown>; // JSON value from backend
}

export interface CreateStatisticRequest {
  name: string;
  category: string;
  statistic_type: string; // "IntegerAmount" | "Habit" | "PointInTime" | "TimeSpans" | "Summing" | "TimeDifference"
  source_statistic_ids?: number[];
}

export interface SetGoalRequest {
  goal_data: Record<string, string>;
}

// Specific goal types for type safety
export interface IntegerAmountGoal {
  target: string;
  is_at_least?: string; // "true" or "false"
}

export interface HabitGoal {
  occurred: string; // "true" or "false"
}

export interface PointInTimeGoal {
  deadline: string; // "HH:MM" or "HH:MM:SS"
}

export interface TimeSpansGoal {
  target: string; // JSON string of TimeFrameList
  efficiency?: string; // "0" to "100"
}

export interface SecondsAmountGoal {
  target: string; // seconds as string
  is_at_least?: string; // "true" or "false"
}
