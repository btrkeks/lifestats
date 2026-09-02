'use client';

import type { StatisticMetadata } from '@/types';
import { parseStatisticDefaultGoal } from '@/lib/schemas';
import { formatGoal } from '@/lib/goalFormat';
import { DayStrip } from '@/features/goals/components/DayStrip';

interface GoalCardProps {
  statistic: StatisticMetadata;
  today: string;
  /** Opens the goal editor; `initialDate` preselects a day in the override panel. */
  onEdit: (initialDate?: string) => void;
}

/** A statistic with an active default goal: goal summary plus the recent-days strip. */
export function GoalCard({ statistic, today, onEdit }: GoalCardProps) {
  const goal = parseStatisticDefaultGoal(statistic);

  return (
    <article className="flex flex-col gap-3 rounded-xl border border-gray-200 bg-white p-4 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <h3 className="truncate font-semibold text-gray-900" title={statistic.name}>
            {statistic.name}
          </h3>
          <p className="mt-0.5 text-sm text-gray-600">
            {goal ? formatGoal(goal) : 'Goal set (unreadable format)'}
          </p>
        </div>
        <button
          type="button"
          onClick={() => onEdit()}
          className="shrink-0 rounded-md border border-gray-300 px-2.5 py-1 text-xs font-medium text-gray-700 transition hover:bg-gray-50"
        >
          Edit goals
        </button>
      </div>

      <DayStrip statisticId={statistic.id} today={today} onEditDay={(date) => onEdit(date)} />

      <p className="text-[11px] text-gray-400">
        #{statistic.id} · {statistic.statistic_type}
      </p>
    </article>
  );
}
