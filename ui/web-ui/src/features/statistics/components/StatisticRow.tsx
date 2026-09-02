import type { StatisticMetadata } from '@/types';

interface StatisticRowProps {
  statistic: StatisticMetadata;
  onSetGoal: () => void;
}

/** Compact row for a statistic without a goal. */
export function StatisticRow({ statistic, onSetGoal }: StatisticRowProps) {
  return (
    <li className="flex items-center justify-between gap-3 py-2">
      <p className="min-w-0 truncate text-sm text-gray-900">
        {statistic.name}
        <span className="ml-2 text-xs text-gray-400">
          #{statistic.id} · {statistic.statistic_type}
        </span>
      </p>
      <button
        type="button"
        onClick={onSetGoal}
        className="shrink-0 text-xs font-medium text-blue-600 hover:underline"
      >
        Set goal
      </button>
    </li>
  );
}
