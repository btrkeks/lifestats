'use client';

import { useState } from 'react';
import { format } from 'date-fns';
import { useStatistic, useDailyGoal, useSetDailyGoal, useDeleteDailyGoal } from '../hooks/useGoals';
import { GoalForm } from './GoalForm';
import type { SetGoalRequest } from '@/types';

interface DailyGoalOverrideProps {
  statisticId: number;
}

/**
 * Controller component for editing daily goal overrides.
 * Fetches data and delegates rendering to the presentational GoalForm.
 */
export function DailyGoalOverride({ statisticId }: DailyGoalOverrideProps) {
  const [selectedDate, setSelectedDate] = useState(format(new Date(), 'yyyy-MM-dd'));

  // Fetch statistic metadata to get the type
  const { data: statistic, isLoading: isLoadingStatistic } = useStatistic(statisticId);

  // Fetch existing goal for the selected date
  const { data: goal, isLoading: isLoadingGoal } = useDailyGoal(
    statisticId,
    selectedDate,
    statistic?.statistic_type ?? 'IntegerAmount'
  );

  const setGoal = useSetDailyGoal(statisticId, selectedDate);
  const deleteGoal = useDeleteDailyGoal(statisticId, selectedDate);

  const handleSubmit = (goalData: Record<string, string>) => {
    const request: SetGoalRequest = { goal_data: goalData };
    setGoal.mutate(request);
  };

  const handleDelete = () => {
    if (confirm(`Delete goal for ${selectedDate}?`)) {
      deleteGoal.mutate();
    }
  };

  if (isLoadingStatistic) {
    return <div className="p-4 text-gray-600">Loading statistic...</div>;
  }

  if (!statistic) {
    return <div className="p-4 text-red-600">Statistic not found</div>;
  }

  return (
    <div className="p-6 bg-white rounded-lg shadow-md">
      <h2 className="text-2xl font-bold mb-4">
        Daily Goal Override - {statistic.name} ({statistic.statistic_type})
      </h2>

      <div className="mb-6">
        <label htmlFor="date" className="block text-sm font-medium text-gray-700">
          Select Date
        </label>
        <input
          id="date"
          type="date"
          value={selectedDate}
          onChange={(e) => setSelectedDate(e.target.value)}
          className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-green-500 focus:ring-green-500 px-3 py-2 border"
        />
      </div>

      {isLoadingGoal && <div className="p-4 text-gray-600">Loading goal...</div>}

      {!isLoadingGoal && (
        <GoalForm
          statisticType={statistic.statistic_type}
          existingGoal={goal}
          onSubmit={handleSubmit}
          onDelete={handleDelete}
          isSubmitting={setGoal.isPending}
          isDeleting={deleteGoal.isPending}
          submitLabel="Set Daily Goal"
          submitColor="green"
        />
      )}

      {setGoal.isError && (
        <p className="text-red-600 text-sm mt-4">Error: {setGoal.error.message}</p>
      )}
      {setGoal.isSuccess && (
        <p className="text-green-600 text-sm mt-4">Goal set successfully!</p>
      )}
    </div>
  );
}
