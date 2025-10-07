import type { StatisticMetadata } from '@/types';
import { StatisticCard } from './StatisticCard';

interface StatisticGridProps {
  statistics: StatisticMetadata[];
  onManageGoal: (statistic: StatisticMetadata) => void;
}

export function StatisticGrid({ statistics, onManageGoal }: StatisticGridProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      {statistics.map((stat) => (
        <StatisticCard
          key={stat.id}
          statistic={stat}
          onManageGoal={() => onManageGoal(stat)}
        />
      ))}
    </div>
  );
}
