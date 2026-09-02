'use client';

import { useEffect } from 'react';
import type { StatisticMetadata } from '@/types';
import { DefaultGoalEditor } from './DefaultGoalEditor';
import { DailyGoalOverride } from './DailyGoalOverride';

interface GoalEditorModalProps {
  statistic: StatisticMetadata;
  /** Preselects this date in the daily override panel. */
  initialDate?: string;
  onClose: () => void;
}

export function GoalEditorModal({ statistic, initialDate, onClose }: GoalEditorModalProps) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="goal-editor-title"
        className="max-h-[90vh] w-full max-w-5xl overflow-y-auto rounded-xl bg-gray-50 shadow-xl"
      >
        <header className="sticky top-0 z-10 flex items-center justify-between border-b border-gray-200 bg-white px-6 py-4">
          <div>
            <h2 id="goal-editor-title" className="text-xl font-bold text-gray-900">
              {statistic.name}
            </h2>
            <p className="text-sm text-gray-500">
              {statistic.category} · {statistic.statistic_type} · #{statistic.id}
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="flex h-8 w-8 items-center justify-center rounded-full text-2xl text-gray-400 transition hover:bg-gray-100 hover:text-gray-700"
          >
            ×
          </button>
        </header>

        <div className="grid gap-6 p-6 lg:grid-cols-2">
          <DefaultGoalEditor statistic={statistic} />
          <DailyGoalOverride statistic={statistic} initialDate={initialDate} />
        </div>
      </div>
    </div>
  );
}
