import axios from 'axios';
import type {
  StatisticMetadata,
  CreateStatisticRequest,
  SetGoalRequest,
  SetDailyGoalRequest,
  GoalResponse,
  GoalResponseRaw,
  DailyGoalState,
  DailyGoalStateRaw,
  DailyAchievementsResponse,
  StatisticType,
} from '@/types';
import { parseDailyGoalState, parseGoalData } from './schemas';

// Base URL for the API - can be configured via environment variable
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000';

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
  const { data } = await api.get<StatisticMetadata[]>('/statistics');
  return data;
}

export async function getLoggableStatistics(): Promise<StatisticMetadata[]> {
  const { data } = await api.get<StatisticMetadata[]>('/statistics/loggable');
  return data;
}

export async function createStatistic(
  request: CreateStatisticRequest
): Promise<StatisticMetadata> {
  const { data } = await api.post<StatisticMetadata>('/statistics', request);
  return data;
}

// ============================================================================
// Goals API (parsed through Zod at the boundary)
// ============================================================================

/** Fetches the default goal; null when none is set. */
export async function getDefaultGoal(
  statisticId: number,
  statisticType: StatisticType
): Promise<GoalResponse | null> {
  const { data } = await api.get<GoalResponseRaw | null>(
    `/statistics/${statisticId}/goals/default`
  );
  if (!data?.goal_data) {
    return null;
  }
  return {
    statistic_type: statisticType,
    goal_data: parseGoalData(statisticType, data.goal_data),
  };
}

export async function setDefaultGoal(
  statisticId: number,
  goal: SetGoalRequest
): Promise<void> {
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
  const { data } = await api.get<DailyGoalStateRaw>(
    `/statistics/${statisticId}/goals/${date}`
  );
  return parseDailyGoalState(statisticType, data);
}

export async function setDailyGoal(
  statisticId: number,
  date: string, // YYYY-MM-DD
  request: SetDailyGoalRequest
): Promise<void> {
  await api.put(`/statistics/${statisticId}/goals/${date}`, request);
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

/** Per-day goal achievement and override state over an inclusive date range. */
export async function getDailyAchievements(
  statisticId: number,
  start: string, // YYYY-MM-DD
  end: string // YYYY-MM-DD
): Promise<DailyAchievementsResponse> {
  const { data } = await api.get<DailyAchievementsResponse>(
    `/statistics/${statisticId}/achievements`,
    { params: { start, end } }
  );
  return data;
}
