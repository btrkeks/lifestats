import { useMutation, useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query';
import {
  getDefaultGoal,
  setDefaultGoal,
  deleteDefaultGoal,
  getDailyGoal,
  setDailyGoal,
  deleteDailyGoal,
  getDailyAchievements,
} from '@/lib/api';
import type { SetGoalRequest, SetDailyGoalRequest, StatisticType } from '@/types';

export const goalKeys = {
  all: (statisticId: number) => ['goal', statisticId] as const,
  default: (statisticId: number) => ['goal', statisticId, 'default'] as const,
  daily: (statisticId: number, date: string) => ['goal', statisticId, date] as const,
  achievements: (statisticId: number) => ['achievements', statisticId] as const,
  achievementsRange: (statisticId: number, start: string, end: string) =>
    ['achievements', statisticId, start, end] as const,
};

/**
 * Any goal change affects the goal queries, the achievement strip and the
 * statistics list (which carries `default_goal`). Returning the promise keeps
 * the mutation pending until the refetches land, so UIs settle in one step.
 */
function invalidateGoalData(queryClient: QueryClient, statisticId: number) {
  return Promise.all([
    queryClient.invalidateQueries({ queryKey: goalKeys.all(statisticId) }),
    queryClient.invalidateQueries({ queryKey: goalKeys.achievements(statisticId) }),
    queryClient.invalidateQueries({ queryKey: ['statistics'] }),
  ]);
}

// ============================================================================
// Default Goal Hooks
// ============================================================================

export function useDefaultGoal(statisticId: number, statisticType: StatisticType) {
  return useQuery({
    queryKey: goalKeys.default(statisticId),
    queryFn: () => getDefaultGoal(statisticId, statisticType),
  });
}

export function useSetDefaultGoal(statisticId: number) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (goal: SetGoalRequest) => setDefaultGoal(statisticId, goal),
    onSuccess: () => invalidateGoalData(queryClient, statisticId),
  });
}

export function useDeleteDefaultGoal(statisticId: number) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => deleteDefaultGoal(statisticId),
    onSuccess: () => invalidateGoalData(queryClient, statisticId),
  });
}

// ============================================================================
// Daily Goal Hooks (the date is a mutation variable so one hook serves any day)
// ============================================================================

export function useDailyGoal(statisticId: number, date: string, statisticType: StatisticType) {
  return useQuery({
    queryKey: goalKeys.daily(statisticId, date),
    queryFn: () => getDailyGoal(statisticId, date, statisticType),
    enabled: date.length > 0,
  });
}

export interface DailyGoalMutation {
  date: string;
  request: SetDailyGoalRequest;
}

export function useSetDailyGoal(statisticId: number) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ date, request }: DailyGoalMutation) =>
      setDailyGoal(statisticId, date, request),
    onSuccess: () => invalidateGoalData(queryClient, statisticId),
  });
}

export function useDeleteDailyGoal(statisticId: number) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (date: string) => deleteDailyGoal(statisticId, date),
    onSuccess: () => invalidateGoalData(queryClient, statisticId),
  });
}

// ============================================================================
// Achievements
// ============================================================================

export function useDailyAchievements(statisticId: number, start: string, end: string) {
  return useQuery({
    queryKey: goalKeys.achievementsRange(statisticId, start, end),
    queryFn: () => getDailyAchievements(statisticId, start, end),
  });
}
