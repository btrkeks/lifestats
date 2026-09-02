'use client';

import { useForm, type FieldValues, type Resolver } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import type { StatisticType, GoalResponse } from '@/types';
import { IntegerAmountFields, HabitFields, PointInTimeFields, TimeSpansFields } from './fields';
import { goalFormDefaultsFor, goalFormSchemaFor, serializeGoalData } from '@/lib/schemas';
import { formatGoal } from '@/lib/goalFormat';
import { ConfirmButton } from '@/components/ConfirmButton';

interface GoalFormProps {
  statisticType: StatisticType;
  existingGoal?: GoalResponse | null;
  onSubmit: (goalData: Record<string, string>) => void;
  onDelete?: () => void;
  onSuppress?: () => void;
  isSubmitting?: boolean;
  isDeleting?: boolean;
  submitLabel?: string;
  variant?: 'primary' | 'success';
}

const SUBMIT_CLASSES: Record<NonNullable<GoalFormProps['variant']>, string> = {
  primary: 'bg-blue-600 hover:bg-blue-700',
  success: 'bg-emerald-600 hover:bg-emerald-700',
};

/**
 * Polymorphic goal form: renders the fields for the statistic type and hands
 * back a serialized (all-strings) goal payload. Presentational only.
 */
export function GoalForm({
  statisticType,
  existingGoal,
  onSubmit,
  onDelete,
  onSuppress,
  isSubmitting = false,
  isDeleting = false,
  submitLabel = 'Save goal',
  variant = 'primary',
}: GoalFormProps) {
  const {
    register,
    handleSubmit,
    control,
    formState: { errors },
  } = useForm<FieldValues>({
    resolver: zodResolver(goalFormSchemaFor(statisticType)) as Resolver<FieldValues>,
    defaultValues: existingGoal ? existingGoal.goal_data.data : goalFormDefaultsFor(statisticType),
  });

  const onFormSubmit = (data: FieldValues) => {
    onSubmit(serializeGoalData(data));
  };

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
        return <TimeSpansFields register={register} errors={errors} control={control} />;
      case 'TimeDifference':
        return (
          <div className="rounded bg-yellow-50 p-4 text-sm text-yellow-800">
            TimeDifference goal editing is not implemented yet.
          </div>
        );
    }
  };

  return (
    <div className="space-y-4">
      {existingGoal && (
        <div className="flex items-center justify-between gap-3 rounded bg-blue-50 px-4 py-3">
          <p className="text-sm text-blue-900">
            <span className="font-semibold">Current:</span> {formatGoal(existingGoal.goal_data)}
          </p>
          {onDelete && (
            <ConfirmButton
              label={isDeleting ? 'Deleting…' : 'Delete'}
              confirmLabel="Confirm delete"
              onConfirm={onDelete}
              disabled={isDeleting}
              className="shrink-0 rounded px-3 py-1 text-sm font-medium text-rose-700 hover:bg-rose-100 disabled:opacity-50"
            />
          )}
        </div>
      )}

      <form onSubmit={handleSubmit(onFormSubmit)} className="space-y-4">
        {renderFields()}

        <div className="flex gap-2">
          {onSuppress && (
            <button
              type="button"
              onClick={onSuppress}
              disabled={isSubmitting}
              className="flex-1 rounded-md bg-amber-500 px-4 py-2 text-white transition hover:bg-amber-600 disabled:opacity-50"
            >
              Suppress goal
            </button>
          )}
          <button
            type="submit"
            disabled={isSubmitting || statisticType === 'TimeDifference'}
            className={`flex-1 rounded-md px-4 py-2 text-white transition disabled:opacity-50 ${SUBMIT_CLASSES[variant]}`}
          >
            {isSubmitting ? 'Saving…' : submitLabel}
          </button>
        </div>
      </form>
    </div>
  );
}
