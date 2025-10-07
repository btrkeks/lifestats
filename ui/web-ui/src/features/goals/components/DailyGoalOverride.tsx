'use client';

import { useState } from 'react';
import { format } from 'date-fns';
import { useDailyGoal, useSetDailyGoal, useDeleteDailyGoal } from '../hooks/useGoals';
import type { SetGoalRequest } from '@/types';

interface DailyGoalOverrideProps {
  statisticId: number;
}

export function DailyGoalOverride({ statisticId }: DailyGoalOverrideProps) {
  const [selectedDate, setSelectedDate] = useState(format(new Date(), 'yyyy-MM-dd'));
  const [target, setTarget] = useState('');
  const [isAtLeast, setIsAtLeast] = useState(true);

  const { data: goal, isLoading } = useDailyGoal(statisticId, selectedDate);
  const setGoal = useSetDailyGoal(statisticId, selectedDate);
  const deleteGoal = useDeleteDailyGoal(statisticId, selectedDate);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    const goalData: SetGoalRequest = {
      goal_data: {
        target,
        is_at_least: isAtLeast ? 'true' : 'false',
      },
    };

    setGoal.mutate(goalData);
  };

  const handleDelete = () => {
    if (confirm(`Delete goal for ${selectedDate}?`)) {
      deleteGoal.mutate();
    }
  };

  return (
    <div className="p-6 bg-white rounded-lg shadow-md">
      <h2 className="text-2xl font-bold mb-4">Daily Goal Override</h2>

      <div className="mb-4">
        <label htmlFor="date" className="block text-sm font-medium text-gray-700">
          Select Date
        </label>
        <input
          id="date"
          type="date"
          value={selectedDate}
          onChange={(e) => setSelectedDate(e.target.value)}
          className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border"
        />
      </div>

      {isLoading && <div className="p-4 text-gray-600">Loading...</div>}

      {goal && (
        <div className="mb-4 p-4 bg-green-50 rounded">
          <p className="text-sm font-semibold text-green-900">Current Goal for {selectedDate}:</p>
          <pre className="text-sm text-green-700 mt-2">
            {JSON.stringify(goal.goal_data, null, 2)}
          </pre>
          <button
            onClick={handleDelete}
            disabled={deleteGoal.isPending}
            className="mt-2 px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 disabled:opacity-50"
          >
            {deleteGoal.isPending ? 'Deleting...' : 'Delete Goal'}
          </button>
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label htmlFor="dailyTarget" className="block text-sm font-medium text-gray-700">
            Target Value
          </label>
          <input
            id="dailyTarget"
            type="text"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border"
            placeholder="e.g., 150"
            required
          />
        </div>

        <div className="flex items-center">
          <input
            id="dailyIsAtLeast"
            type="checkbox"
            checked={isAtLeast}
            onChange={(e) => setIsAtLeast(e.target.checked)}
            className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
          />
          <label htmlFor="dailyIsAtLeast" className="ml-2 block text-sm text-gray-700">
            Is At Least (vs. At Most)
          </label>
        </div>

        <button
          type="submit"
          disabled={setGoal.isPending}
          className="w-full px-4 py-2 bg-green-600 text-white rounded-md hover:bg-green-700 disabled:opacity-50"
        >
          {setGoal.isPending ? 'Setting Goal...' : 'Set Daily Goal'}
        </button>

        {setGoal.isError && (
          <p className="text-red-600 text-sm">Error: {setGoal.error.message}</p>
        )}
        {setGoal.isSuccess && (
          <p className="text-green-600 text-sm">Goal set successfully!</p>
        )}
      </form>
    </div>
  );
}
