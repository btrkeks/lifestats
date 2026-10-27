import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
  type QueryClient,
} from '@tanstack/react-query';
import {
  deleteDailyGoal,
  getDailyAchievements,
  getDefaultGoal,
  getStreak,
  setDailyGoal,
  setDailyGoalRange,
  setDefaultGoal,
} from '@/lib/api';
import type {
  SetDailyGoalRangeRequest,
  SetDailyGoalRequest,
  SetGoalRequest,
  StatisticMetadata,
} from '@/types';
import { toDays, type Day } from '../day';

export const goalKeys = {
  default: (statisticId: number) => ['goal', statisticId, 'default'] as const,
  achievements: (statisticId: number) => ['achievements', statisticId] as const,
  achievementsRange: (statisticId: number, start: string, end: string) =>
    ['achievements', statisticId, start, end] as const,
  streak: (statisticId: number) => ['streak', statisticId] as const,
};

/**
 * Any goal change affects the default goal, every achievement range, the
 * streak and the statistics list (which carries `default_goal`). Returning the
 * promise keeps the mutation pending until the refetches land, so UIs settle
 * in one step.
 */
function invalidateGoalData(queryClient: QueryClient, statisticId: number) {
  return Promise.all([
    queryClient.invalidateQueries({ queryKey: ['goal', statisticId] }),
    queryClient.invalidateQueries({ queryKey: goalKeys.achievements(statisticId) }),
    queryClient.invalidateQueries({ queryKey: goalKeys.streak(statisticId) }),
    queryClient.invalidateQueries({ queryKey: ['statistics'] }),
  ]);
}

type StatisticRef = Pick<StatisticMetadata, 'id' | 'statistic_type'>;

export function useDefaultGoal(statistic: StatisticRef) {
  return useQuery({
    queryKey: goalKeys.default(statistic.id),
    queryFn: () => getDefaultGoal(statistic.id, statistic.statistic_type),
  });
}

function daysQuery(statistic: StatisticRef, start: string, end: string, today: string) {
  return {
    queryKey: goalKeys.achievementsRange(statistic.id, start, end),
    queryFn: () => getDailyAchievements(statistic.id, statistic.statistic_type, start, end),
    select: (days: Awaited<ReturnType<typeof getDailyAchievements>>): Day[] => toDays(days, today),
  };
}

/** Classified days from `start` to `end` (inclusive). Disabled while `start > end`. */
export function useDays(
  statistic: StatisticRef,
  {
    start,
    end,
    today,
    enabled = true,
  }: { start: string; end: string; today: string; enabled?: boolean }
) {
  return useQuery({ ...daysQuery(statistic, start, end, today), enabled: enabled && start <= end });
}

/** Today's day for each statistic, in the same order. */
export function useTodays(statistics: StatisticRef[], today: string) {
  return useQueries({
    queries: statistics.map((statistic) => daysQuery(statistic, today, today, today)),
  });
}

export function useStreaks(statisticIds: number[]) {
  return useQueries({
    queries: statisticIds.map((id) => ({
      queryKey: goalKeys.streak(id),
      queryFn: () => getStreak(id),
      select: (streak: Awaited<ReturnType<typeof getStreak>>) => streak.current_streak_days,
    })),
  });
}

export function useSetDefaultGoal(statisticId: number) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (goal: SetGoalRequest) => setDefaultGoal(statisticId, goal),
    onSuccess: () => invalidateGoalData(queryClient, statisticId),
  });
}

export interface DailyGoalMutation {
  date: string;
  request: SetDailyGoalRequest;
}

export function useSetDailyGoal(statisticId: number) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ date, request }: DailyGoalMutation) => setDailyGoal(statisticId, date, request),
    onSuccess: () => invalidateGoalData(queryClient, statisticId),
  });
}

export function useSetDailyGoalRange(statisticId: number) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: SetDailyGoalRangeRequest) => setDailyGoalRange(statisticId, request),
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
