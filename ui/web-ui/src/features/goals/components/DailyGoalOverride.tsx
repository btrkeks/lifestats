'use client';

import { useState } from 'react';
import { format } from 'date-fns';
import { useStatistic, useDailyGoal, useSetDailyGoal, useDeleteDailyGoal } from '../hooks/useGoals';
import { GoalForm } from './GoalForm';
import { createGoalRequest, createSuppressedRequest } from '@/lib/schemas';
import type { SetDailyGoalRequest } from '@/types';

interface DailyGoalOverrideProps {
  statisticId: number;
}

/**
 * Controller component for editing daily goal overrides.
 * Fetches data and delegates rendering to the presentational GoalForm.
 */
export function DailyGoalOverride({ statisticId }: DailyGoalOverrideProps) {
  const [selectedDate, setSelectedDate] = useState(format(new Date(), 'yyyy-MM-dd'));
  const [isSuppressed, setIsSuppressed] = useState(false);

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

  // Update suppressed state when date changes
  const handleDateChange = (newDate: string) => {
    setSelectedDate(newDate);
    setIsSuppressed(false);
  };

  const handleSubmit = (goalData: Record<string, string>) => {
    const request: SetDailyGoalRequest = createGoalRequest(goalData);
    setGoal.mutate(request);
    setIsSuppressed(false);
  };

  const handleSuppress = () => {
    if (confirm(`Suppress goal for ${selectedDate}? The default goal will not apply on this date.`)) {
      const request: SetDailyGoalRequest = createSuppressedRequest();
      setGoal.mutate(request);
      setIsSuppressed(true);
    }
  };

  const handleDelete = () => {
    if (confirm(`Delete goal for ${selectedDate}?`)) {
      deleteGoal.mutate();
      setIsSuppressed(false);
    }
  };

  if (isLoadingStatistic) {
    return <div className="p-4 text-black">Loading statistic...</div>;
  }

  if (!statistic) {
    return <div className="p-4 text-red-600">Statistic not found</div>;
  }

  return (
    <div className="p-6 bg-white rounded-lg shadow-md">
      <h2 className="text-2xl font-bold mb-4 text-black">
        Daily Goal Override - {statistic.name} ({statistic.statistic_type})
      </h2>

      <div className="mb-6">
        <label htmlFor="date" className="block text-sm font-medium text-black">
          Select Date
        </label>
        <input
          id="date"
          type="date"
          value={selectedDate}
          onChange={(e) => handleDateChange(e.target.value)}
          className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-green-500 focus:ring-green-500 px-3 py-2 border text-black"
        />
      </div>

      {isLoadingGoal && <div className="p-4 text-black">Loading goal...</div>}

      {!isLoadingGoal && isSuppressed && (
        <div className="p-4 bg-yellow-50 border border-yellow-200 rounded-md mb-6">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-medium text-yellow-800">Goal Suppressed for {selectedDate}</p>
              <p className="text-sm text-yellow-700 mt-1">The default goal will not apply on this date.</p>
            </div>
            <button
              onClick={handleDelete}
              disabled={deleteGoal.isPending}
              className="px-3 py-1 text-sm font-medium text-yellow-700 bg-yellow-100 hover:bg-yellow-200 rounded disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {deleteGoal.isPending ? 'Removing...' : 'Remove Suppression'}
            </button>
          </div>
        </div>
      )}

      {!isLoadingGoal && !isSuppressed && (
        <GoalForm
          statisticType={statistic.statistic_type}
          existingGoal={goal}
          onSubmit={handleSubmit}
          onDelete={handleDelete}
          onSuppress={handleSuppress}
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
