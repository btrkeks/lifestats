import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { createStatistic, getAllStatistics, getLoggableStatistics } from '@/lib/api';
import type { CreateStatisticRequest } from '@/types';

export function useStatistics() {
  return useQuery({
    queryKey: ['statistics'],
    queryFn: getAllStatistics,
  });
}

export function useLoggableStatistics(enabled: boolean) {
  return useQuery({
    queryKey: ['statistics', 'loggable'],
    queryFn: getLoggableStatistics,
    enabled,
  });
}

export function useCreateStatistic() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (request: CreateStatisticRequest) => createStatistic(request),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['statistics'] }),
  });
}
