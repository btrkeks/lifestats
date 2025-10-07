'use client';

import { useState } from 'react';
import { useDefaultGoal, useSetDefaultGoal, useDeleteDefaultGoal } from '../hooks/useGoals';
import type { SetGoalRequest } from '@/types';

interface DefaultGoalEditorProps {
  statisticId: number;
}

export function DefaultGoalEditor({ statisticId }: DefaultGoalEditorProps) {
  const { data: goal, isLoading } = useDefaultGoal(statisticId);
  const setGoal = useSetDefaultGoal(statisticId);
  const deleteGoal = useDeleteDefaultGoal(statisticId);

  const [target, setTarget] = useState('');
  const [isAtLeast, setIsAtLeast] = useState(true);

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
    if (confirm('Are you sure you want to delete the default goal?')) {
      deleteGoal.mutate();
    }
  };

  if (isLoading) {
    return <div className="p-4 text-gray-600">Loading...</div>;
  }

  return (
    <div className="p-6 bg-white rounded-lg shadow-md">
      <h2 className="text-2xl font-bold mb-4">Default Goal</h2>

      {goal && (
        <div className="mb-4 p-4 bg-blue-50 rounded">
          <p className="text-sm font-semibold text-blue-900">Current Goal:</p>
          <pre className="text-sm text-blue-700 mt-2">
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
          <label htmlFor="target" className="block text-sm font-medium text-gray-700">
            Target Value
          </label>
          <input
            id="target"
            type="text"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            className="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 px-3 py-2 border"
            placeholder="e.g., 100"
            required
          />
        </div>

        <div className="flex items-center">
          <input
            id="isAtLeast"
            type="checkbox"
            checked={isAtLeast}
            onChange={(e) => setIsAtLeast(e.target.checked)}
            className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
          />
          <label htmlFor="isAtLeast" className="ml-2 block text-sm text-gray-700">
            Is At Least (vs. At Most)
          </label>
        </div>

        <button
          type="submit"
          disabled={setGoal.isPending}
          className="w-full px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50"
        >
          {setGoal.isPending ? 'Setting Goal...' : 'Set Default Goal'}
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
