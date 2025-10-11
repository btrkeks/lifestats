'use client';

import { useMemo } from 'react';
import { Controller, useForm } from 'react-hook-form';
import { z } from 'zod';
import { zodResolver } from '@hookform/resolvers/zod';
import type {
  CreateStatisticRequest,
  StatisticMetadata,
  StatisticType,
} from '@/types';

const formSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  category: z.string().min(1, 'Category is required'),
  source_statistic_ids: z.array(z.number()),
});

type FormValues = z.infer<typeof formSchema>;

interface CreateStatisticFormProps {
  statisticType: StatisticType;
  onSubmit: (payload: CreateStatisticRequest) => void;
  isSubmitting: boolean;
  loggableStatistics: StatisticMetadata[];
  isLoadingSources: boolean;
  requiresSources: boolean;
}

export function CreateStatisticForm({
  statisticType,
  onSubmit,
  isSubmitting,
  loggableStatistics,
  isLoadingSources,
  requiresSources,
}: CreateStatisticFormProps) {
  const schema = useMemo(
    () =>
      formSchema.superRefine((values, ctx) => {
        if (!requiresSources) {
          return;
        }

        if (statisticType === 'Summing' && values.source_statistic_ids.length === 0) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Select at least one source statistic.',
            path: ['source_statistic_ids'],
          });
        }

        if (statisticType === 'TimeDifference' && values.source_statistic_ids.length !== 2) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            message: 'Select exactly two source statistics to calculate a difference.',
            path: ['source_statistic_ids'],
          });
        }
      }),
    [requiresSources, statisticType]
  );

  const {
    handleSubmit,
    register,
    control,
    formState: { errors },
  } = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: '',
      category: '',
      source_statistic_ids: [],
    },
  });

  const submitHandler = handleSubmit((values) => {
    const payload: CreateStatisticRequest = {
      name: values.name,
      category: values.category,
      statistic_type: statisticType,
    };

    if (requiresSources) {
      payload.source_statistic_ids = values.source_statistic_ids;
    }

    onSubmit(payload);
  });

  const insufficientSources =
    requiresSources &&
    ((statisticType === 'Summing' && loggableStatistics.length === 0) ||
      (statisticType === 'TimeDifference' && loggableStatistics.length < 2));

  const disableSubmission =
    isSubmitting || (requiresSources && (isLoadingSources || insufficientSources));

  return (
    <form className="space-y-6" onSubmit={submitHandler}>
      <div>
        <label className="block text-sm font-medium text-gray-700" htmlFor="name">
          Statistic name
        </label>
        <input
          id="name"
          type="text"
          {...register('name')}
          className="mt-2 w-full rounded-lg border border-gray-300 px-3 py-2 text-gray-900 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          placeholder="Morning push-ups"
        />
        {errors.name && <p className="mt-1 text-sm text-red-600">{errors.name.message}</p>}
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-700" htmlFor="category">
          Category
        </label>
        <input
          id="category"
          type="text"
          {...register('category')}
          className="mt-2 w-full rounded-lg border border-gray-300 px-3 py-2 text-gray-900 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          placeholder="Fitness"
        />
        {errors.category && (
          <p className="mt-1 text-sm text-red-600">{errors.category.message}</p>
        )}
      </div>

      {requiresSources && (
        <div>
          <label className="block text-sm font-medium text-gray-700" htmlFor="source_statistic_ids">
            Source statistics
          </label>
          <p className="mt-1 text-sm text-gray-500">
            Select the statistics that should be used as inputs for this derived statistic.
          </p>

          {isLoadingSources ? (
            <p className="mt-3 text-sm text-gray-500">Loading available statistics…</p>
          ) : insufficientSources ? (
            <p className="mt-3 text-sm text-red-600">
              {statisticType === 'Summing'
                ? 'No statistics are available to use as sources yet. Create a loggable statistic first.'
                : 'You need at least two loggable statistics to calculate a time difference.'}
            </p>
          ) : (
            <Controller
              name="source_statistic_ids"
              control={control}
              render={({ field }) => (
                <select
                  id="source_statistic_ids"
                  multiple
                  className="mt-3 h-40 w-full rounded-lg border border-gray-300 px-3 py-2 text-gray-900 shadow-sm focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
                  value={field.value.map((id) => id.toString())}
                  onChange={(event) => {
                    const selectedValues = Array.from(event.target.selectedOptions).map((option) =>
                      Number(option.value)
                    );
                    field.onChange(selectedValues);
                  }}
                >
                  {loggableStatistics.map((statistic) => (
                    <option key={statistic.id} value={statistic.id}>
                      {statistic.name} ({statistic.category})
                    </option>
                  ))}
                </select>
              )}
            />
          )}

          {errors.source_statistic_ids && (
            <p className="mt-1 text-sm text-red-600">{errors.source_statistic_ids.message}</p>
          )}
        </div>
      )}

      <div className="flex items-center justify-end gap-3">
        <button
          type="submit"
          className="rounded-md bg-blue-600 px-4 py-2 font-medium text-white shadow-sm transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:bg-blue-300"
          disabled={disableSubmission}
        >
          {isSubmitting ? 'Creating…' : 'Create statistic'}
        </button>
      </div>
    </form>
  );
}
