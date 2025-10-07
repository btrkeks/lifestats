import type { StatisticMetadata } from '@/types';

interface StatisticCardProps {
  statistic: StatisticMetadata;
  onManageGoal: () => void;
}

export function StatisticCard({ statistic, onManageGoal }: StatisticCardProps) {
  return (
    <div className="p-6 bg-white rounded-lg shadow-md hover:shadow-lg transition-shadow">
      <div className="mb-4">
        <h3 className="text-xl font-semibold text-gray-900">{statistic.name}</h3>
        <p className="text-sm text-gray-600 mt-1">{statistic.category}</p>
        <p className="text-xs text-gray-500 mt-1">Type: {statistic.statistic_type}</p>
      </div>
      <button
        onClick={onManageGoal}
        className="w-full px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
      >
        Manage Goal
      </button>
    </div>
  );
}
