'use client';

import { useState } from 'react';
import type { StatisticMetadata } from '@/types';
import { useStatistics } from '@/features/statistics/hooks/useStatistics';
import { GoalCard } from '@/features/statistics/components/GoalCard';
import { StatisticRow } from '@/features/statistics/components/StatisticRow';
import { GoalEditorModal } from '@/features/goals/components/GoalEditorModal';
import { DayLegend } from '@/features/goals/components/DayLegend';
import { groupByCategory, splitByGoal } from '@/lib/statisticGroups';
import { useToday } from '@/lib/hooks/useToday';

interface EditorTarget {
  statistic: StatisticMetadata;
  initialDate?: string;
}

export default function DashboardPage() {
  const [editing, setEditing] = useState<EditorTarget | null>(null);
  const today = useToday();
  const { data: statistics, isLoading, error } = useStatistics();

  if (isLoading) {
    return <p className="py-12 text-center text-gray-600">Loading statistics…</p>;
  }
  if (error) {
    return (
      <div className="rounded-lg border border-rose-200 bg-rose-50 p-6 text-rose-800">
        Error loading statistics: {error instanceof Error ? error.message : 'Unknown error'}
      </div>
    );
  }

  const { withGoal, withoutGoal } = splitByGoal(statistics ?? []);

  return (
    <>
      <section>
        <header className="mb-4 flex flex-wrap items-end justify-between gap-3">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">Active goals</h1>
            <p className="mt-1 text-sm text-gray-500">
              Click a day to suppress its goal or set a custom one.
            </p>
          </div>
          <DayLegend />
        </header>

        {withGoal.length === 0 && (
          <div className="rounded-lg border border-dashed border-gray-300 bg-white p-8 text-center text-gray-500">
            No active goals yet. Pick a statistic below and set one.
          </div>
        )}

        {groupByCategory(withGoal).map((group) => (
          <div key={group.category} className="mb-8">
            <h2 className="mb-3 text-xs font-semibold uppercase tracking-wide text-gray-500">
              {group.category}
            </h2>
            <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
              {group.statistics.map((statistic) => (
                <GoalCard
                  key={statistic.id}
                  statistic={statistic}
                  today={today}
                  onEdit={(initialDate) => setEditing({ statistic, initialDate })}
                />
              ))}
            </div>
          </div>
        ))}
      </section>

      <details className="rounded-xl border border-gray-200 bg-white">
        <summary className="cursor-pointer px-4 py-3 font-medium text-gray-800">
          Other statistics ({withoutGoal.length})
        </summary>
        <div className="border-t border-gray-100 px-4 pb-4">
          {withoutGoal.length === 0 && (
            <p className="pt-4 text-sm text-gray-500">Every statistic has a goal.</p>
          )}
          {groupByCategory(withoutGoal).map((group) => (
            <div key={group.category} className="pt-4">
              <h3 className="text-xs font-semibold uppercase tracking-wide text-gray-500">
                {group.category}
              </h3>
              <ul className="divide-y divide-gray-100">
                {group.statistics.map((statistic) => (
                  <StatisticRow
                    key={statistic.id}
                    statistic={statistic}
                    onSetGoal={() => setEditing({ statistic })}
                  />
                ))}
              </ul>
            </div>
          ))}
        </div>
      </details>

      {editing && (
        <GoalEditorModal
          statistic={editing.statistic}
          initialDate={editing.initialDate}
          onClose={() => setEditing(null)}
        />
      )}
    </>
  );
}
