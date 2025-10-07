import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  getDefaultGoal,
  setDefaultGoal,
  deleteDefaultGoal,
  getDailyGoal,
  setDailyGoal,
  deleteDailyGoal,
  getStatistic,
} from "@/lib/api";
import type { SetGoalRequest, StatisticType } from "@/types";

// ============================================================================
// Statistic Hooks
// ============================================================================

export function useStatistic(statisticId: number) {
  return useQuery({
    queryKey: ["statistic", statisticId],
    queryFn: () => getStatistic(statisticId),
    enabled: !!statisticId,
  });
}

// ============================================================================
// Default Goal Hooks
// ============================================================================

export function useDefaultGoal(
  statisticId: number,
  statisticType: StatisticType,
) {
  return useQuery({
    queryKey: ["goal", statisticId, "default"],
    queryFn: () => getDefaultGoal(statisticId, statisticType!),
    enabled: !!statisticId && !!statisticType,
  });
}

export function useSetDefaultGoal(statisticId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (goal: SetGoalRequest) => setDefaultGoal(statisticId, goal),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["goal", statisticId, "default"],
      });
    },
  });
}

export function useDeleteDefaultGoal(statisticId: number) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => deleteDefaultGoal(statisticId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["goal", statisticId, "default"],
      });
    },
  });
}

// ============================================================================
// Daily Goal Hooks
// ============================================================================

export function useDailyGoal(
  statisticId: number,
  date: string,
  statisticType: StatisticType,
) {
  return useQuery({
    queryKey: ["goal", statisticId, date],
    queryFn: () => getDailyGoal(statisticId, date, statisticType),
    enabled: !!statisticId && !!date && !!statisticType,
  });
}

export function useSetDailyGoal(statisticId: number, date: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (goal: SetGoalRequest) => setDailyGoal(statisticId, date, goal),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["goal", statisticId, date] });
    },
  });
}

export function useDeleteDailyGoal(statisticId: number, date: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => deleteDailyGoal(statisticId, date),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["goal", statisticId, date] });
    },
  });
}
