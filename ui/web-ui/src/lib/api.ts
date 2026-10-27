import axios from 'axios';
import type {
  StatisticMetadata,
  CreateStatisticRequest,
  SetGoalRequest,
  SetDailyGoalRequest,
  DefaultGoal,
  GoalResponseRaw,
  SetDailyGoalRangeRequest,
  StreakResponse,
  DailyGoalState,
  DailyGoalStateRaw,
  AchievementDay,
  DailyAchievementsResponse,
  StatisticType,
} from '@/types';
import { parseDailyGoalState, parseGoalData, statisticListSchema } from './schemas';

// Base URL for the API - can be configured via environment variable
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

/** A readable message for a failed request (the API answers `{ "error": "..." }`). */
export function errorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const body: unknown = error.response?.data;
    if (
      typeof body === 'object' &&
      body !== null &&
      'error' in body &&
      typeof body.error === 'string'
    ) {
      return body.error;
    }
    return error.message;
  }
  return error instanceof Error ? error.message : 'Something went wrong.';
}

const api = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
  },
});

// ============================================================================
// Statistics API
// ============================================================================

export async function getAllStatistics(): Promise<StatisticMetadata[]> {
  const { data } = await api.get<unknown>('/statistics');
  return statisticListSchema.parse(data);
}

export async function getLoggableStatistics(): Promise<StatisticMetadata[]> {
  const { data } = await api.get<unknown>('/statistics/loggable');
  return statisticListSchema.parse(data);
}

export async function createStatistic(request: CreateStatisticRequest): Promise<StatisticMetadata> {
  const { data } = await api.post<StatisticMetadata>('/statistics', request);
  return data;
}

// ============================================================================
// Goals API (parsed through Zod at the boundary)
// ============================================================================

/** Fetches the current default goal and its start; null when none is set. */
export async function getDefaultGoal(
  statisticId: number,
  statisticType: StatisticType
): Promise<DefaultGoal | null> {
  const { data } = await api.get<GoalResponseRaw | null>(
    `/statistics/${statisticId}/goals/default`
  );
  if (!data?.goal_data) {
    return null;
  }
  return {
    goal: parseGoalData(statisticType, data.goal_data),
    validSince: data.valid_since,
  };
}

export async function setDefaultGoal(statisticId: number, goal: SetGoalRequest): Promise<void> {
  await api.put(`/statistics/${statisticId}/goals/default`, goal);
}

export async function deleteDefaultGoal(statisticId: number): Promise<void> {
  await api.delete(`/statistics/${statisticId}/goals/default`);
}

/** Fetches the daily override state (not_set / suppressed / goal) for a date. */
export async function getDailyGoal(
  statisticId: number,
  date: string, // YYYY-MM-DD
  statisticType: StatisticType
): Promise<DailyGoalState> {
  const { data } = await api.get<DailyGoalStateRaw>(`/statistics/${statisticId}/goals/${date}`);
  return parseDailyGoalState(statisticType, data);
}

export async function setDailyGoal(
  statisticId: number,
  date: string, // YYYY-MM-DD
  request: SetDailyGoalRequest
): Promise<void> {
  await api.put(`/statistics/${statisticId}/goals/${date}`, request);
}

/** Sets the same override on every day from `start` to `end`, all or nothing. */
export async function setDailyGoalRange(
  statisticId: number,
  request: SetDailyGoalRangeRequest
): Promise<void> {
  await api.put(`/statistics/${statisticId}/goals/range`, request);
}

export async function deleteDailyGoal(
  statisticId: number,
  date: string // YYYY-MM-DD
): Promise<void> {
  await api.delete(`/statistics/${statisticId}/goals/${date}`);
}

// ============================================================================
// Achievements API
// ============================================================================

/**
 * Per-day goal achievement, override state, logged value and applicable goal
 * over an inclusive date range, with each day's goal parsed.
 */
export async function getDailyAchievements(
  statisticId: number,
  statisticType: StatisticType,
  start: string, // YYYY-MM-DD
  end: string // YYYY-MM-DD
): Promise<AchievementDay[]> {
  const { data } = await api.get<DailyAchievementsResponse>(
    `/statistics/${statisticId}/achievements`,
    { params: { start, end } }
  );
  return data.days.map((day) => ({
    ...day,
    goal: day.goal ? parseGoalData(statisticType, day.goal) : null,
  }));
}

/** Current goal-achievement streak in days. */
export async function getStreak(statisticId: number): Promise<StreakResponse> {
  const { data } = await api.get<StreakResponse>(`/statistics/${statisticId}/streak`);
  return data;
}
