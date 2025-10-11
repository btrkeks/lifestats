import axios from 'axios';
import type {
  StatisticMetadata,
  CreateStatisticRequest,
  SetGoalRequest,
  GoalResponse,
  GoalResponseRaw,
  StatisticType,
} from '@/types';
import { parseGoalData } from './schemas';

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
  const response = await api.get('/statistics');
  return response.data;
}

export async function getLoggableStatistics(): Promise<StatisticMetadata[]> {
  const response = await api.get('/statistics/loggable');
  return response.data;
}

export async function getStatistic(id: number): Promise<StatisticMetadata> {
  const response = await api.get(`/statistics/${id}`);
  return response.data;
}

export async function createStatistic(
  data: CreateStatisticRequest
): Promise<StatisticMetadata> {
  const response = await api.post('/statistics', data);
  return response.data;
}

export async function deleteStatistic(id: number): Promise<void> {
  await api.delete(`/statistics/${id}`);
}

// ============================================================================
// Goals API (Type-Safe with Zod Parsing)
// ============================================================================

/**
 * Fetches and parses a default goal with type safety.
 * Returns null if no goal exists.
 */
export async function getDefaultGoal(
  statisticId: number,
  statisticType: StatisticType
): Promise<GoalResponse | null> {
  try {
    const response = await api.get<GoalResponseRaw>(
      `/statistics/${statisticId}/goals/default`
    );

    if (!response.data || !response.data.goal_data) {
      return null;
    }

    // Parse raw data through Zod schemas for type safety
    const parsedGoalData = parseGoalData(statisticType, response.data.goal_data);

    return {
      statistic_type: statisticType,
      goal_data: parsedGoalData,
    };
  } catch (error) {
    if (axios.isAxiosError(error) && error.response?.status === 404) {
      return null;
    }
    throw error;
  }
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

/**
 * Fetches and parses a daily goal with type safety.
 * Returns null if no goal exists.
 */
export async function getDailyGoal(
  statisticId: number,
  date: string, // YYYY-MM-DD format
  statisticType: StatisticType
): Promise<GoalResponse | null> {
  try {
    const response = await api.get<GoalResponseRaw>(
      `/statistics/${statisticId}/goals/${date}`
    );

    if (!response.data || !response.data.goal_data) {
      return null;
    }

    // Parse raw data through Zod schemas for type safety
    const parsedGoalData = parseGoalData(statisticType, response.data.goal_data);

    return {
      statistic_type: statisticType,
      goal_data: parsedGoalData,
    };
  } catch (error) {
    if (axios.isAxiosError(error) && error.response?.status === 404) {
      return null;
    }
    throw error;
  }
}

export async function setDailyGoal(
  statisticId: number,
  date: string, // YYYY-MM-DD format
  goal: SetGoalRequest
): Promise<void> {
  await api.put(`/statistics/${statisticId}/goals/${date}`, goal);
}

export async function deleteDailyGoal(
  statisticId: number,
  date: string // YYYY-MM-DD format
): Promise<void> {
  await api.delete(`/statistics/${statisticId}/goals/${date}`);
}
