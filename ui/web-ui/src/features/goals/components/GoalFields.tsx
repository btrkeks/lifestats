'use client';

import { useId } from 'react';
import { Controller, useFieldArray, type FieldValues, type UseFormReturn } from 'react-hook-form';
import { Clock, Plus, X } from 'lucide-react';
import { cn } from 'cn';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Segmented } from '@/components/Segmented';
import type { Measure } from '@/lib/measure';
import type { StatisticType } from '@/types';

type GoalForm = UseFormReturn<FieldValues>;

export function FieldLabel({ htmlFor, children }: { htmlFor?: string; children: React.ReactNode }) {
  return (
    <label htmlFor={htmlFor} className="mt-3 mb-1.5 block text-xs font-[550] text-ink-2 first:mt-1">
      {children}
    </label>
  );
}

function FieldError({ form, name }: { form: GoalForm; name: string }) {
  const error = form.formState.errors[name];
  if (!error?.message) return null;
  return <p className="mt-1 text-xs text-missed">{String(error.message)}</p>;
}

/** Seconds edited as hours + minutes. */
function DurationInput({ form, compact }: { form: GoalForm; compact: boolean }) {
  return (
    <Controller
      control={form.control}
      name="target"
      render={({ field }) => {
        const total = Number(field.value) || 0;
        const hours = Math.floor(total / 3600);
        const minutes = Math.round((total % 3600) / 60);
        const set = (h: number, m: number) =>
          field.onChange(Math.max(0, h) * 3600 + Math.max(0, m) * 60);
        return (
          <span className="flex items-center gap-1.5">
            <Input
              aria-label="Hours"
              type="number"
              min={0}
              value={hours}
              onChange={(event) => set(Number(event.target.value), minutes)}
              className={cn('text-right font-semibold', compact ? 'w-14' : 'w-16')}
            />
            <span className="text-muted-foreground">h</span>
            <Input
              aria-label="Minutes"
              type="number"
              min={0}
              max={59}
              value={minutes}
              onChange={(event) => set(hours, Number(event.target.value))}
              className={cn('text-right font-semibold', compact ? 'w-14' : 'w-16')}
            />
            <span className="text-muted-foreground">min</span>
          </span>
        );
      }}
    />
  );
}

function DirectionToggle({ form }: { form: GoalForm }) {
  return (
    <Controller
      control={form.control}
      name="is_at_least"
      render={({ field }) => (
        <Segmented
          label="Direction"
          value={String(field.value) === 'false' ? 'most' : 'least'}
          onChange={(value) => field.onChange(value === 'least')}
          options={[
            { value: 'least', label: 'At least' },
            { value: 'most', label: 'At most' },
          ]}
          className="w-44"
        />
      )}
    />
  );
}

function AmountFields({
  form,
  measure,
  compact,
}: {
  form: GoalForm;
  measure: Measure;
  compact: boolean;
}) {
  const id = useId();
  const atLeast = String(form.watch('is_at_least')) !== 'false';
  const amount =
    measure.kind === 'duration' ? (
      <DurationInput form={form} compact={compact} />
    ) : (
      <span className="flex items-center gap-2">
        <Input
          id={id}
          type="number"
          {...form.register('target', { valueAsNumber: true })}
          className={cn('font-semibold', compact ? 'w-24' : 'w-32')}
        />
        {measure.kind === 'count' && measure.unit && (
          <span className="text-muted-foreground">{measure.unit}</span>
        )}
      </span>
    );

  if (compact) {
    const sign = (
      <span
        className="text-[12.5px] text-muted-foreground"
        aria-label={atLeast ? 'At least' : 'At most'}
      >
        {atLeast ? '≥' : '≤'}
      </span>
    );
    if (measure.kind === 'duration') {
      return (
        <span className="flex items-center gap-1.5">
          {sign}
          {amount}
        </span>
      );
    }
    return (
      <span className="flex h-[38px] w-[110px] shrink-0 items-center gap-1.5 rounded-lg border border-input bg-card px-2.5 focus-within:border-brand focus-within:ring-3 focus-within:ring-brand-tint">
        {sign}
        <input
          id={id}
          type="number"
          aria-label="Target"
          {...form.register('target', { valueAsNumber: true })}
          className="w-full min-w-0 bg-transparent text-sm font-semibold tabular outline-none"
        />
      </span>
    );
  }
  return (
    <>
      <FieldLabel>Each day</FieldLabel>
      <DirectionToggle form={form} />
      <div className="mt-2">{amount}</div>
      <FieldError form={form} name="target" />
    </>
  );
}

function DeadlineField({ form, compact }: { form: GoalForm; compact: boolean }) {
  const id = useId();
  const input = (
    <span
      className={cn(
        'flex h-9 items-center gap-1.5 rounded-lg border border-input bg-card px-2.5 focus-within:border-brand focus-within:ring-3 focus-within:ring-brand-tint',
        compact ? 'h-[38px] w-[110px] shrink-0' : 'h-10 w-full'
      )}
    >
      {compact ? (
        <span className="text-[12.5px] text-muted-foreground">by</span>
      ) : (
        <Clock className="size-4 text-muted-foreground" />
      )}
      <input
        id={id}
        type="time"
        aria-label="Deadline"
        {...form.register('deadline')}
        className={cn(
          'w-full bg-transparent font-semibold tabular outline-none',
          compact ? 'text-sm' : 'text-[15px]'
        )}
      />
    </span>
  );
  if (compact) return input;
  return (
    <>
      <FieldLabel htmlFor={id}>Done by</FieldLabel>
      {input}
      <FieldError form={form} name="deadline" />
    </>
  );
}

function HabitField({ form }: { form: GoalForm }) {
  return (
    <Controller
      control={form.control}
      name="occurred"
      render={({ field }) => (
        <Segmented
          label="Habit goal"
          value={String(field.value) === 'false' ? 'skip' : 'done'}
          onChange={(value) => field.onChange(value === 'done')}
          options={[
            { value: 'done', label: 'Done' },
            { value: 'skip', label: 'Not done' },
          ]}
          className="w-44"
        />
      )}
    />
  );
}

function WindowsFields({ form }: { form: GoalForm }) {
  const { fields, append, remove } = useFieldArray({ control: form.control, name: 'target' });
  const efficiencyId = useId();
  return (
    <>
      <FieldLabel>Time windows</FieldLabel>
      <div className="flex flex-col gap-2">
        {fields.map((field, index) => (
          <div key={field.id} className="flex items-center gap-2">
            <Input
              type="time"
              aria-label={`Window ${index + 1} start`}
              {...form.register(`target.${index}.start`)}
              className="w-[96px] text-sm font-semibold"
            />
            <span className="text-faint">–</span>
            <Input
              type="time"
              aria-label={`Window ${index + 1} end`}
              {...form.register(`target.${index}.end`)}
              className="w-[96px] text-sm font-semibold"
            />
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label={`Remove window ${index + 1}`}
              onClick={() => remove(index)}
              disabled={fields.length === 1}
            >
              <X />
            </Button>
          </div>
        ))}
      </div>
      <Button
        type="button"
        variant="ghost"
        size="xs"
        className="mt-1.5"
        onClick={() => append({ start: '21:00', end: '21:30' })}
      >
        <Plus /> Add window
      </Button>
      <FieldError form={form} name="target" />
      <FieldLabel htmlFor={efficiencyId}>Efficiency</FieldLabel>
      <div className="flex items-center gap-2.5">
        <span className="flex h-9 w-[76px] items-center rounded-lg border border-input bg-card px-2.5 focus-within:border-brand focus-within:ring-3 focus-within:ring-brand-tint">
          <input
            id={efficiencyId}
            type="number"
            min={0}
            max={100}
            {...form.register('efficiency', { valueAsNumber: true })}
            className="w-full bg-transparent font-semibold tabular outline-none"
          />
          <span className="text-muted-foreground">%</span>
        </span>
        <span className="text-[12.5px] leading-snug text-muted-foreground">
          of the windows must be spent on it
        </span>
      </div>
      <FieldError form={form} name="efficiency" />
    </>
  );
}

/** Whether `GoalFields` has a compact one-line form for this statistic type. */
export function hasInlineFields(statisticType: StatisticType): boolean {
  return statisticType !== 'TimeSpans' && statisticType !== 'TimeDifference';
}

/**
 * The goal inputs for a statistic type. `compact` renders a single line for
 * the selection bar and the day popover (not available for time spans).
 */
export function GoalFields({
  form,
  statisticType,
  measure,
  compact = false,
}: {
  form: GoalForm;
  statisticType: StatisticType;
  measure: Measure;
  compact?: boolean;
}) {
  switch (statisticType) {
    case 'IntegerAmount':
    case 'Summing':
    case 'TimeDifference':
      return <AmountFields form={form} measure={measure} compact={compact} />;
    case 'PointInTime':
      return <DeadlineField form={form} compact={compact} />;
    case 'Habit':
      return <HabitField form={form} />;
    case 'TimeSpans':
      return <WindowsFields form={form} />;
  }
}
