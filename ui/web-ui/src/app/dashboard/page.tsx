'use client';

import { useState } from 'react';
import Link from 'next/link';
import type { StatisticMetadata } from '@/types';
import { useStatistics } from '@/features/statistics/hooks/useStatistics';
import { StatisticGrid } from '@/features/statistics/components/StatisticGrid';
import { GoalEditorModal } from '@/features/goals/components/GoalEditorModal';

export default function DashboardPage() {
  const [editingStatistic, setEditingStatistic] = useState<StatisticMetadata | null>(null);

  const { data: statistics, isLoading, error } = useStatistics();

  const handleOpenModal = (statistic: StatisticMetadata) => {
    setEditingStatistic(statistic);
  };

  const handleCloseModal = () => {
    setEditingStatistic(null);
  };

  return (
    <div className="min-h-screen bg-gray-50 p-8">
      <div className="max-w-7xl mx-auto">
        <div className="mb-8">
          <Link href="/" className="text-blue-600 hover:text-blue-800 hover:underline">
            ← Back to Home
          </Link>
        </div>

        <header className="mb-8">
          <h1 className="text-4xl font-bold text-gray-900">Statistics Dashboard</h1>
          <p className="text-gray-600 mt-2">
            Manage goals for all your statistics in one place
          </p>
        </header>

        {isLoading && (
          <div className="text-center py-12">
            <p className="text-gray-600 text-lg">Loading statistics...</p>
          </div>
        )}

        {error && (
          <div className="bg-red-50 border border-red-200 rounded-lg p-6">
            <p className="text-red-800">
              Error loading statistics: {error instanceof Error ? error.message : 'Unknown error'}
            </p>
          </div>
        )}

        {statistics && statistics.length === 0 && (
          <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-6">
            <p className="text-yellow-800">
              No statistics found. Create some statistics using the API first.
            </p>
          </div>
        )}

        {statistics && statistics.length > 0 && (
          <StatisticGrid statistics={statistics} onManageGoal={handleOpenModal} />
        )}

        {editingStatistic && (
          <GoalEditorModal statistic={editingStatistic} onClose={handleCloseModal} />
        )}
      </div>
    </div>
  );
}
