'use client';

import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import type { StatisticType, GoalResponse } from '@/types';
import {
  IntegerAmountFields,
  HabitFields,
  PointInTimeFields,
  TimeSpansFields,
} from './fields';
import {
  integerAmountGoalSchema,
  habitGoalSchema,
  pointInTimeGoalSchema,
  timeSpansGoalSchema,
  serializeGoalData,
} from '@/lib/schemas';

interface GoalFormProps {
  statisticType: StatisticType;
  existingGoal?: GoalResponse | null;
  onSubmit: (goalData: Record<string, string>) => void;
  onDelete?: () => void;
  isSubmitting?: boolean;
  isDeleting?: boolean;
  submitLabel?: string;
  submitColor?: string;
}

/**
 * Polymorphic goal form component that renders the appropriate fields
 * based on the statistic type. This is a "dumb" presentational component
 * that doesn't know about data fetching - it just renders UI and calls callbacks.
 */
export function GoalForm({
  statisticType,
  existingGoal,
  onSubmit,
  onDelete,
  isSubmitting = false,
  isDeleting = false,
  submitLabel = 'Set Goal',
  submitColor = 'blue',
}: GoalFormProps) {
  // Determine the schema based on statistic type
  const getSchema = () => {
    switch (statisticType) {
      case 'IntegerAmount':
      case 'Summing':
        return integerAmountGoalSchema;
      case 'Habit':
        return habitGoalSchema;
      case 'PointInTime':
        return pointInTimeGoalSchema;
      case 'TimeSpans':
        return timeSpansGoalSchema;
      default:
        // For types we haven't implemented yet (e.g., TimeDifference)
        return integerAmountGoalSchema;
    }
  };

  // Get default values from existing goal if available
  const getDefaultValues = () => {
    if (!existingGoal) {
      // Default values for new goals
      switch (statisticType) {
        case 'IntegerAmount':
        case 'Summing':
          return { target: 0, is_at_least: true };
        case 'Habit':
          return { occurred: true };
        case 'PointInTime':
          return { deadline: '12:00:00' };
        case 'TimeSpans':
          return { target: [{ start: '09:00', end: '17:00' }], efficiency: 80 };
        default:
          return {};
      }
    }

    if (statisticType === 'TimeSpans' && existingGoal.goal_data.type === 'TimeSpans') {
      const data = existingGoal.goal_data.data;
      const maybeStringTarget = data.target as unknown;

      if (typeof maybeStringTarget === 'string') {
        try {
          const parsedTarget = JSON.parse(maybeStringTarget);
          return { ...(data as Record<string, unknown>), target: parsedTarget };
        } catch (error) {
          console.warn('Failed to parse TimeSpans target JSON', error);
          return data as unknown as Record<string, unknown>;
        }
      }

      return data as unknown as Record<string, unknown>;
    }

    // Extract data from existing goal
    return existingGoal.goal_data.data;
  };

  const {
    register,
    handleSubmit,
    control,
    formState: { errors },
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
  } = useForm<Record<string, any>>({
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    resolver: zodResolver(getSchema()) as any,
    defaultValues: getDefaultValues(),
  });

  const onFormSubmit = (data: Record<string, unknown>) => {
    const payload = { ...data };

    if (statisticType === 'TimeSpans' && Array.isArray(payload.target)) {
      payload.target = JSON.stringify(payload.target);
    }

    // Serialize to backend format (all strings)
    const serialized = serializeGoalData(payload);
    onSubmit(serialized);
  };

  // Render the appropriate fields based on statistic type
  const renderFields = () => {
    switch (statisticType) {
      case 'IntegerAmount':
      case 'Summing':
        return <IntegerAmountFields register={register} errors={errors} />;
      case 'Habit':
        return <HabitFields register={register} errors={errors} />;
      case 'PointInTime':
        return <PointInTimeFields register={register} errors={errors} />;
      case 'TimeSpans':
        return (
          <TimeSpansFields register={register} errors={errors} control={control} />
        );
      case 'TimeDifference':
        return (
          <div className="p-4 bg-yellow-50 rounded text-sm text-yellow-800">
            TimeDifference goal editing not yet implemented
          </div>
        );
      default:
        return (
          <div className="p-4 bg-red-50 rounded text-sm text-red-800">
            Unknown statistic type: {statisticType}
          </div>
        );
    }
  };

  return (
    <div className="space-y-4">
      {existingGoal && (
        <div className="p-4 bg-blue-50 rounded">
          <p className="text-sm font-semibold text-blue-900">Current Goal:</p>
          <pre className="text-sm text-blue-700 mt-2">
            {JSON.stringify(existingGoal.goal_data.data, null, 2)}
          </pre>
          {onDelete && (
            <button
              type="button"
              onClick={onDelete}
              disabled={isDeleting}
              className="mt-2 px-4 py-2 bg-red-500 text-white rounded hover:bg-red-600 disabled:opacity-50"
            >
              {isDeleting ? 'Deleting...' : 'Delete Goal'}
            </button>
          )}
        </div>
      )}

      <form onSubmit={handleSubmit(onFormSubmit)} className="space-y-4">
        {renderFields()}

        <button
          type="submit"
          disabled={isSubmitting}
          className={`w-full px-4 py-2 bg-${submitColor}-600 text-white rounded-md hover:bg-${submitColor}-700 disabled:opacity-50`}
        >
          {isSubmitting ? 'Setting Goal...' : submitLabel}
        </button>
      </form>
    </div>
  );
}
