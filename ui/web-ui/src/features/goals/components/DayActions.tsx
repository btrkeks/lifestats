'use client';

import { useState, type KeyboardEvent, type ReactNode } from 'react';
import type { FieldValues } from 'react-hook-form';
import { Ban, RotateCcw, Target, X } from 'lucide-react';
import { cn } from 'cn';
import { Button } from '@/components/ui/button';
import { Kbd } from '@/components/ui/kbd';
import { formatDay } from '@/lib/dates';
import { formatClock, formatGoalTarget } from '@/lib/goalFormat';
import { formatValue, type Measure } from '@/lib/measure';
import { serializeGoalData } from '@/lib/schemas';
import type { GoalData, StatisticType } from '@/types';
import { DAY_STATUS_LABEL, type Day, type DayStatus } from '../day';
import { useGoalForm } from '../useGoalForm';
import { GoalFields, hasInlineFields } from './GoalFields';

export const STATUS_DOT: Record<DayStatus, string> = {
  met: 'bg-met',
  missed: 'bg-missed',
  open: 'bg-open',
  suppressed: 'bg-hatch shadow-[inset_0_0_0_1px_#D8D4CB]',
  none: 'bg-card shadow-[inset_0_0_0_1px_#DDD9D0]',
  future: 'bg-card shadow-[inset_0_0_0_1px_#DDD9D0]',
};

/** "244 of 400 cards (61%)", "22:37 against 22:05 (74%)", "nothing logged". */
export function describeDayResult(day: Day, measure: Measure): string | null {
  const percent = day.percentage_achieved === null ? '' : ` (${day.percentage_achieved}%)`;
  if (day.value === null) return day.goal ? `nothing logged${percent}` : null;
  const value = formatValue(measure, day.value);
  const goal = day.status === 'suppressed' || day.status === 'none' ? null : day.goal;
  if (!goal) return `${value} logged`;
  switch (goal.type) {
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference': {
      const unit = measure.kind === 'count' && measure.unit ? ` ${measure.unit}` : '';
      return `${value} of ${formatGoalTarget(goal, measure)}${unit}${percent}`;
    }
    case 'PointInTime':
      return `${value}, goal by ${formatClock(goal.data.deadline)}${percent}`;
    case 'Habit':
    case 'TimeSpans':
      return `${value}${percent}`;
  }
}

function Action({
  icon,
  title,
  hint,
  shortcut,
  disabled,
  onClick,
}: {
  icon: ReactNode;
  title: string;
  hint: string;
  shortcut?: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  const inverted = 'group-hover/action:text-white/75 group-focus-visible/action:text-white/75';
  return (
    <button
      type="button"
      data-day-action
      onClick={onClick}
      disabled={disabled}
      className="group/action flex items-center gap-[11px] rounded-lg px-[9px] py-2 text-left outline-none hover:bg-ink hover:text-white focus-visible:bg-ink focus-visible:text-white disabled:opacity-50"
    >
      <span className={cn('text-ink-2 [&_svg]:size-[18px]', inverted)}>{icon}</span>
      <span className="flex flex-1 flex-col">
        <b className="text-[13px] font-semibold">{title}</b>
        <small className={cn('text-[11.5px] text-muted-foreground', inverted)}>{hint}</small>
      </span>
      {shortcut && (
        <Kbd
          className={cn(
            'group-hover/action:border-white/30 group-hover/action:bg-transparent group-focus-visible/action:border-white/30 group-focus-visible/action:bg-transparent',
            inverted
          )}
        >
          {shortcut}
        </Kbd>
      )}
    </button>
  );
}

function CustomGoalForm({
  statisticType,
  measure,
  initial,
  pending,
  onCancel,
  onSave,
}: {
  statisticType: StatisticType;
  measure: Measure;
  initial: GoalData | null;
  pending: boolean;
  onCancel: () => void;
  onSave: (goalData: Record<string, string>) => void;
}) {
  const form = useGoalForm(statisticType, initial);
  const submit = form.handleSubmit((values: FieldValues) => onSave(serializeGoalData(values)));
  return (
    <form onSubmit={submit} className="px-3.5 pb-3">
      <p className="mb-2 text-xs font-[550] text-ink-2">Goal for this day only</p>
      <GoalFields
        form={form}
        statisticType={statisticType}
        measure={measure}
        compact={hasInlineFields(statisticType)}
      />
      <div className="mt-3 flex justify-end gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
          Back
        </Button>
        <Button type="submit" size="sm" disabled={pending}>
          {pending ? 'Saving…' : 'Set goal'}
        </Button>
      </div>
    </form>
  );
}

/**
 * The day popover's content: what happened that day and the one-click
 * actions (Suppress goal, Set custom goal…, Remove override).
 */
export function DayActions({
  day,
  measure,
  statisticType,
  defaultGoal,
  pending,
  error,
  onSuppress,
  onRemoveOverride,
  onSetGoal,
  onClose,
}: {
  day: Day;
  measure: Measure;
  statisticType: StatisticType;
  defaultGoal: GoalData | null;
  pending: boolean;
  error: string | null;
  onSuppress: () => void;
  onRemoveOverride: () => void;
  onSetGoal: (goalData: Record<string, string>) => void;
  onClose: () => void;
}) {
  const [editing, setEditing] = useState(false);
  const result = describeDayResult(day, measure);
  const canSuppress = day.override !== 'suppressed';

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (editing || event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === 's' && canSuppress && !pending) {
      event.preventDefault();
      onSuppress();
    } else if (event.key === 'g') {
      event.preventDefault();
      setEditing(true);
    }
  };

  return (
    <div onKeyDown={onKeyDown}>
      <div className="flex items-start justify-between pt-3 pr-3 pb-2.5 pl-3.5">
        <div>
          <p className="text-[14.5px] font-[650]">{formatDay(day.date)}</p>
          <p className="mt-0.5 flex items-center gap-1.5 text-xs text-muted-foreground">
            <span className={cn('size-2 rounded-[2px]', STATUS_DOT[day.status])} aria-hidden />
            {day.override === 'goal' ? 'Custom goal · ' : ''}
            {DAY_STATUS_LABEL[day.status]}
            {result && ` · ${result}`}
          </p>
        </div>
        <Button variant="ghost" size="icon-sm" aria-label="Close" onClick={onClose}>
          <X />
        </Button>
      </div>
      {editing ? (
        <CustomGoalForm
          statisticType={statisticType}
          measure={measure}
          initial={day.goal ?? defaultGoal}
          pending={pending}
          onCancel={() => setEditing(false)}
          onSave={onSetGoal}
        />
      ) : (
        <div className="flex flex-col gap-0.5 px-1.5 pb-1.5">
          {canSuppress && (
            <Action
              icon={<Ban />}
              title="Suppress goal"
              hint="Excuse this day; it won't count"
              shortcut="S"
              disabled={pending}
              onClick={onSuppress}
            />
          )}
          <Action
            icon={<Target />}
            title={day.override === 'goal' ? 'Change custom goal…' : 'Set custom goal…'}
            hint="A different target for this day only"
            shortcut="G"
            disabled={pending}
            onClick={() => setEditing(true)}
          />
          {day.override !== 'none' && (
            <Action
              icon={<RotateCcw />}
              title="Remove override"
              hint={defaultGoal ? 'Use the default goal again' : 'No goal applies again'}
              disabled={pending}
              onClick={onRemoveOverride}
            />
          )}
        </div>
      )}
      {error && <p className="px-3.5 pb-2 text-xs text-missed">{error}</p>}
      <p className="border-t border-line-2 px-3.5 py-2 text-[11.5px] text-faint">
        Shift-click another day to select a range
      </p>
    </div>
  );
}
