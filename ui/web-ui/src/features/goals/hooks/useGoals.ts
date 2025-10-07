import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  getDefaultGoal,
  setDefaultGoal,
  deleteDefaultGoal,
  getDailyGoal,
  setDailyGoal,
  deleteDailyGoal,
} from '@/lib/api';
import type { SetGoalRequest } from '@/types';

// ============================================================================
// Default Goal Hooks
// ============================================================================

export function useDefaultGoal(statisticId: number) {
  return useQuery({
    queryKey: ['goal', statisticId, 'default'],
    queryFn: () => getDefaultGoal(statisticId),
    enabled: !!statisticId,
  });
}

export function useSetDefaultGoal(statisticId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (goal: SetGoalRequest) => setDefaultGoal(statisticId, goal),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['goal', statisticId, 'default'] });
    },
  });
}

export function useDeleteDefaultGoal(statisticId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => deleteDefaultGoal(statisticId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['goal', statisticId, 'default'] });
    },
  });
}

// ============================================================================
// Daily Goal Hooks
// ============================================================================

export function useDailyGoal(statisticId: number, date: string) {
  return useQuery({
    queryKey: ['goal', statisticId, date],
    queryFn: () => getDailyGoal(statisticId, date),
    enabled: !!statisticId && !!date,
  });
}

export function useSetDailyGoal(statisticId: number, date: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (goal: SetGoalRequest) => setDailyGoal(statisticId, date, goal),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['goal', statisticId, date] });
    },
  });
}

export function useDeleteDailyGoal(statisticId: number, date: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => deleteDailyGoal(statisticId, date),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['goal', statisticId, date] });
    },
  });
}
