'use client';

import type { StatisticMetadata } from '@/types';
import { MutationStatus } from '@/components/MutationStatus';
import { useDefaultGoal, useSetDefaultGoal, useDeleteDefaultGoal } from '../hooks/useGoals';
import { GoalForm } from './GoalForm';

interface DefaultGoalEditorProps {
  statistic: StatisticMetadata;
}

/** Controller for the default goal: fetches it and wires the mutations into GoalForm. */
export function DefaultGoalEditor({ statistic }: DefaultGoalEditorProps) {
  const { data: goal, isLoading, isError } = useDefaultGoal(statistic.id, statistic.statistic_type);
  const setGoal = useSetDefaultGoal(statistic.id);
  const deleteGoal = useDeleteDefaultGoal(statistic.id);

  return (
    <section className="rounded-lg bg-white p-5 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900">Default goal</h3>
      <p className="mb-4 text-sm text-gray-500">Applies every day unless a daily override exists.</p>

      {isLoading ? (
        <p className="text-sm text-gray-500">Loading…</p>
      ) : (
        <GoalForm
          statisticType={statistic.statistic_type}
          existingGoal={goal ?? null}
          onSubmit={(goalData) => setGoal.mutate({ goal_data: goalData })}
          onDelete={goal ? () => deleteGoal.mutate() : undefined}
          isSubmitting={setGoal.isPending}
          isDeleting={deleteGoal.isPending}
          submitLabel="Save default goal"
        />
      )}

      {isError && (
        <p className="mt-3 text-sm text-amber-700">
          The stored goal could not be read; saving will overwrite it.
        </p>
      )}
      <MutationStatus
        error={setGoal.error ?? deleteGoal.error}
        isSuccess={setGoal.isSuccess || deleteGoal.isSuccess}
        successText={deleteGoal.isSuccess ? 'Default goal deleted.' : 'Default goal saved.'}
      />
    </section>
  );
}
