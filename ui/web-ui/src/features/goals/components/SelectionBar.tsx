'use client';

import type { FieldValues } from 'react-hook-form';
import { Ban, Target, X } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import type { Measure } from '@/lib/measure';
import { serializeGoalData } from '@/lib/schemas';
import type { GoalData, StatisticType } from '@/types';
import type { Day } from '../day';
import { describeSelection, formatSelectionSpan } from '../selection';
import { useGoalForm } from '../useGoalForm';
import { GoalFields, hasInlineFields } from './GoalFields';

export function SelectionBar({
  from,
  to,
  days,
  measure,
  statisticType,
  defaultGoal,
  pending,
  error,
  onSetGoal,
  onSuppress,
  onClear,
}: {
  from: string;
  to: string;
  days: Day[];
  measure: Measure;
  statisticType: StatisticType;
  defaultGoal: GoalData | null;
  pending: boolean;
  error: string | null;
  onSetGoal: (goalData: Record<string, string>) => void;
  onSuppress: () => void;
  onClear: () => void;
}) {
  const form = useGoalForm(statisticType, defaultGoal);
  const count = days.length;
  const submit = form.handleSubmit((values: FieldValues) => onSetGoal(serializeGoalData(values)));
  const setLabel = `Set goal for ${count} ${count === 1 ? 'day' : 'days'}`;

  return (
    <div className="mt-2.5">
      <form
        onSubmit={submit}
        aria-label="Selected days"
        className="flex h-[58px] items-center gap-1.5 rounded-xl border border-brand-line bg-card pr-1.5 pl-2 shadow-[0_10px_28px_-14px_rgba(59,76,214,0.45),0_2px_6px_rgba(28,27,25,0.05)]"
      >
        <span className="flex size-[42px] shrink-0 flex-col items-center justify-center rounded-[9px] bg-brand text-[17px] leading-none font-[650] text-white tabular">
          {count}
          <small className="mt-0.5 text-[9.5px] font-semibold tracking-[0.04em] uppercase opacity-80">
            {count === 1 ? 'day' : 'days'}
          </small>
        </span>
        <span className="mr-0.5 flex min-w-0 flex-1 flex-col leading-[1.3]">
          <b className="text-[13.5px] font-[620] whitespace-nowrap">
            {formatSelectionSpan(from, to)}
          </b>
          <span className="truncate text-[11.5px] text-muted-foreground">
            Selected · {describeSelection(days)}
          </span>
        </span>
        {hasInlineFields(statisticType) ? (
          <>
            <GoalFields form={form} statisticType={statisticType} measure={measure} compact />
            <Button type="submit" size="lg" disabled={pending}>
              <Target data-icon="inline-start" />
              {setLabel}
            </Button>
          </>
        ) : (
          <Popover>
            <PopoverTrigger render={<Button type="button" size="lg" disabled={pending} />}>
              <Target data-icon="inline-start" />
              {setLabel}…
            </PopoverTrigger>
            <PopoverContent side="top" className="w-[300px] p-3.5">
              <GoalFields form={form} statisticType={statisticType} measure={measure} />
              <Button
                type="button"
                className="mt-3 w-full"
                disabled={pending}
                onClick={() => submit()}
              >
                {setLabel}
              </Button>
            </PopoverContent>
          </Popover>
        )}
        <Button type="button" variant="outline" size="lg" disabled={pending} onClick={onSuppress}>
          <Ban data-icon="inline-start" />
          Suppress goal
        </Button>
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label="Clear selection (Esc)"
          onClick={onClear}
        >
          <X />
        </Button>
      </form>
      {error && <p className="mt-1.5 px-2 text-xs text-missed">{error}</p>}
    </div>
  );
}
