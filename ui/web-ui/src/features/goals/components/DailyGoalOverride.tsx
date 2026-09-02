'use client';

import { useState } from 'react';
import type { StatisticMetadata } from '@/types';
import { MutationStatus } from '@/components/MutationStatus';
import { useToday } from '@/lib/hooks/useToday';
import { formatDayLabel } from '@/lib/dates';
import { createGoalRequest, createSuppressedRequest } from '@/lib/schemas';
import { useDailyGoal, useSetDailyGoal, useDeleteDailyGoal } from '../hooks/useGoals';
import { GoalForm } from './GoalForm';

interface DailyGoalOverrideProps {
  statistic: StatisticMetadata;
  /** Date to open with (YYYY-MM-DD); defaults to today. */
  initialDate?: string;
}

/**
 * Controller for per-date overrides. The override state (not set / suppressed /
 * custom goal) always comes from the server, so it survives reloads.
 */
export function DailyGoalOverride({ statistic, initialDate }: DailyGoalOverrideProps) {
  const today = useToday();
  const [selectedDate, setSelectedDate] = useState(initialDate ?? today);

  const { data: state, isLoading, isError } = useDailyGoal(
    statistic.id,
    selectedDate,
    statistic.statistic_type
  );
  const setGoal = useSetDailyGoal(statistic.id);
  const deleteGoal = useDeleteDailyGoal(statistic.id);

  const submit = (goalData: Record<string, string>) =>
    setGoal.mutate({ date: selectedDate, request: createGoalRequest(goalData) });
  const suppress = () =>
    setGoal.mutate({ date: selectedDate, request: createSuppressedRequest() });
  const removeOverride = () => deleteGoal.mutate(selectedDate);

  return (
    <section className="rounded-lg bg-white p-5 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900">Daily override</h3>
      <p className="mb-4 text-sm text-gray-500">Replaces the default goal on one date only.</p>

      <label htmlFor="override-date" className="block text-sm font-medium text-gray-700">
        Date
      </label>
      <input
        id="override-date"
        type="date"
        value={selectedDate}
        onChange={(event) => setSelectedDate(event.target.value)}
        className="mt-1 mb-5 block w-full rounded-md border border-gray-300 px-3 py-2 text-gray-900 shadow-sm focus:border-emerald-500 focus:ring-emerald-500"
      />

      {isLoading && <p className="text-sm text-gray-500">Loading…</p>}
      {isError && <p className="text-sm text-rose-600">Couldn’t load the override for this date.</p>}

      {state?.kind === 'suppressed' && (
        <div className="flex items-center justify-between gap-3 rounded-md border border-amber-200 bg-amber-50 p-4">
          <div>
            <p className="text-sm font-medium text-amber-900">
              Goal suppressed on {formatDayLabel(selectedDate)}
            </p>
            <p className="mt-1 text-sm text-amber-800">No goal applies on this date.</p>
          </div>
          <button
            type="button"
            onClick={removeOverride}
            disabled={deleteGoal.isPending}
            className="shrink-0 rounded bg-amber-100 px-3 py-1 text-sm font-medium text-amber-900 hover:bg-amber-200 disabled:opacity-50"
          >
            {deleteGoal.isPending ? 'Removing…' : 'Remove suppression'}
          </button>
        </div>
      )}

      {state && state.kind !== 'suppressed' && (
        <>
          {state.kind === 'not_set' && (
            <p className="mb-3 text-sm text-gray-500">
              No override on this date — the default goal applies.
            </p>
          )}
          <GoalForm
            key={selectedDate}
            statisticType={statistic.statistic_type}
            existingGoal={state.kind === 'goal' ? state.goal : null}
            onSubmit={submit}
            onDelete={state.kind === 'goal' ? removeOverride : undefined}
            onSuppress={suppress}
            isSubmitting={setGoal.isPending}
            isDeleting={deleteGoal.isPending}
            submitLabel="Save daily goal"
            variant="success"
          />
        </>
      )}

      <MutationStatus
        error={setGoal.error ?? deleteGoal.error}
        isSuccess={setGoal.isSuccess || deleteGoal.isSuccess}
        successText="Daily override saved."
      />
    </section>
  );
}
