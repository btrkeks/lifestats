'use client';

import { useForm, type FieldValues, type Resolver } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import type { GoalData, StatisticType } from '@/types';
import { formatClock } from '@/lib/goalFormat';
import { goalFormDefaultsFor, goalFormSchemaFor } from '@/lib/schemas';

/** Form values for an existing goal; clock times as "HH:MM" for <input type="time">. */
export function goalFormValues(goal: GoalData): FieldValues {
  switch (goal.type) {
    case 'PointInTime':
      return { deadline: formatClock(goal.data.deadline) };
    case 'TimeSpans':
      return {
        efficiency: goal.data.efficiency,
        target: goal.data.target.map((frame) => ({
          start: formatClock(frame.start),
          end: formatClock(frame.end),
        })),
      };
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference':
    case 'Habit':
      return { ...goal.data };
  }
}

/** A goal form for `statisticType`, starting from `initial` (or sensible defaults). */
export function useGoalForm(statisticType: StatisticType, initial: GoalData | null) {
  return useForm<FieldValues>({
    resolver: zodResolver(goalFormSchemaFor(statisticType)) as Resolver<FieldValues>,
    defaultValues: initial ? goalFormValues(initial) : goalFormDefaultsFor(statisticType),
  });
}
