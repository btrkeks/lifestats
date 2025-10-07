import { useQuery } from '@tanstack/react-query';
import { getAllStatistics } from '@/lib/api';

export function useStatistics() {
  return useQuery({
    queryKey: ['statistics'],
    queryFn: getAllStatistics,
  });
}
