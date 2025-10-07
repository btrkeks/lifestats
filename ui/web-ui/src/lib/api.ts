import axios from 'axios';
import type {
  StatisticMetadata,
  CreateStatisticRequest,
  SetGoalRequest,
  GoalResponse,
} from '@/types';

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
// Goals API
// ============================================================================

export async function getDefaultGoal(
  statisticId: number
): Promise<GoalResponse | null> {
  const response = await api.get(`/statistics/${statisticId}/goals/default`);
  return response.data;
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

export async function getDailyGoal(
  statisticId: number,
  date: string // YYYY-MM-DD format
): Promise<GoalResponse | null> {
  const response = await api.get(`/statistics/${statisticId}/goals/${date}`);
  return response.data;
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
