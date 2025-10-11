"use client";

import {
  useStatistic,
  useDefaultGoal,
  useSetDefaultGoal,
  useDeleteDefaultGoal,
} from "../hooks/useGoals";
import { GoalForm } from "./GoalForm";
import type { SetGoalRequest } from "@/types";

interface DefaultGoalEditorProps {
  statisticId: number;
}

/**
 * Controller component for editing default goals.
 * Fetches data and delegates rendering to the presentational GoalForm.
 */
export function DefaultGoalEditor({ statisticId }: DefaultGoalEditorProps) {
  // Fetch statistic metadata to get the type
  const { data: statistic, isLoading: isLoadingStatistic } =
    useStatistic(statisticId);

  // Fetch existing goal (only if we know the statistic type)
  const { data: goal, isLoading: isLoadingGoal } = useDefaultGoal(
    statisticId,
    statistic?.statistic_type,
  );

  const setGoal = useSetDefaultGoal(statisticId);
  const deleteGoal = useDeleteDefaultGoal(statisticId);

  const handleSubmit = (goalData: Record<string, string>) => {
    const request: SetGoalRequest = { goal_data: goalData };
    setGoal.mutate(request);
  };

  const handleDelete = () => {
    if (confirm("Are you sure you want to delete the default goal?")) {
      deleteGoal.mutate();
    }
  };

  if (isLoadingStatistic || isLoadingGoal) {
    return <div className="p-4 text-black">Loading...</div>;
  }

  if (!statistic) {
    return <div className="p-4 text-red-600">Statistic not found</div>;
  }

  return (
    <div className="p-6 bg-white rounded-lg shadow-md">
      <h2 className="text-2xl font-bold mb-4">
        Default Goal - {statistic.name} ({statistic.statistic_type})
      </h2>

      <GoalForm
        statisticType={statistic.statistic_type}
        existingGoal={goal}
        onSubmit={handleSubmit}
        onDelete={handleDelete}
        isSubmitting={setGoal.isPending}
        isDeleting={deleteGoal.isPending}
        submitLabel="Set Default Goal"
        submitColor="blue"
      />

      {setGoal.isError && (
        <p className="text-red-600 text-sm mt-4">
          Error: {setGoal.error.message}
        </p>
      )}
      {setGoal.isSuccess && (
        <p className="text-green-600 text-sm mt-4">Goal set successfully!</p>
      )}
    </div>
  );
}
