'use client';

import { useEffect, useState, type ReactNode } from 'react';
import { format, parseISO } from 'date-fns';
import type { FieldValues } from 'react-hook-form';
import { CalendarDays, ChevronDown, Info, Pencil, X } from 'lucide-react';
import { cn } from 'cn';
import { Button } from '@/components/ui/button';
import { Calendar } from '@/components/ui/calendar';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { formatDay, formatDayMonth, formatDayWithYear, shiftIsoDate, toIsoDate } from '@/lib/dates';
import { describeGoal, formatGoal } from '@/lib/goalFormat';
import { formatCount, formatSeconds, type Measure } from '@/lib/measure';
import { formGoal, serializeGoalData } from '@/lib/schemas';
import { STATISTIC_TYPES } from '@/lib/statisticTypes';
import type { DefaultGoal, GoalData, StatisticMetadata } from '@/types';
import { startOfWeekIso } from '@/features/calendar/grid';
import type { Day } from '../day';
import type { Effect, EffectKind } from '../defaultGoalPreview';
import { useGoalForm } from '../useGoalForm';
import { FieldLabel, GoalFields } from './GoalFields';

/** Default goal starts the backend never dated. */
const UNDATED = '1970-01-01';

export function RailCard({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <section className={cn('rounded-xl border border-line bg-card px-4 pt-3.5 pb-4', className)}>
      {children}
    </section>
  );
}

function CardHeading({ children, aside }: { children: ReactNode; aside?: ReactNode }) {
  return (
    <div className="mb-2 flex min-h-[26px] items-center justify-between">
      <h3 className="text-[11.5px] font-semibold tracking-[0.05em] text-muted-foreground uppercase">
        {children}
      </h3>
      {aside}
    </div>
  );
}

function BigGoal({ goal, measure }: { goal: GoalData; measure: Measure }) {
  const text = formatGoal(goal, measure);
  const unit = measure.kind === 'count' && measure.unit ? ` ${measure.unit}` : '';
  const main = unit && text.endsWith(unit) ? text.slice(0, -unit.length) : text;
  return (
    <p className="mb-1.5 text-[28px] font-[650] tracking-[-0.02em] tabular">
      {main}
      {unit && (
        <span className="text-[15px] font-[550] tracking-normal text-muted-foreground">{unit}</span>
      )}
    </p>
  );
}

export function DefaultGoalCard({
  current,
  measure,
  onChange,
}: {
  current: DefaultGoal;
  measure: Measure;
  onChange: (() => void) | null;
}) {
  return (
    <RailCard>
      <CardHeading
        aside={
          onChange && (
            <Button variant="outline" size="xs" onClick={onChange}>
              <Pencil data-icon="inline-start" /> Change
            </Button>
          )
        }
      >
        Default goal
      </CardHeading>
      <BigGoal goal={current.goal} measure={measure} />
      <p className="leading-normal text-muted-foreground">
        {describeGoal(current.goal, measure)}{' '}
        {current.validSince === UNDATED ? (
          'Applies every day without a daily override.'
        ) : (
          <>
            Applies from <b className="font-semibold text-ink">{formatDay(current.validSince)}</b>{' '}
            to every day without a daily override.
          </>
        )}
      </p>
    </RailCard>
  );
}

export function OverridesCard({
  days,
  month,
  measure,
  pendingDate,
  onRemove,
}: {
  days: Day[];
  /** "October" */
  month: string;
  measure: Measure;
  pendingDate: string | null;
  onRemove: ((date: string) => void) | null;
}) {
  const overrides = days.filter((day) => day.override !== 'none');
  return (
    <RailCard>
      <CardHeading
        aside={
          <span className="text-xs text-muted-foreground">
            {overrides.length ? `${month} · ${overrides.length}` : month}
          </span>
        }
      >
        Daily overrides
      </CardHeading>
      {overrides.length === 0 ? (
        <p className="text-[12.5px] leading-normal text-muted-foreground">
          None this month. Click a day in the calendar to suppress it or give it its own goal.
        </p>
      ) : (
        <ul>
          {overrides.map((day) => (
            <li
              key={day.date}
              className="flex h-[34px] items-center gap-2 border-t border-line-2 first:border-t-0"
            >
              <span className="flex-1 text-[13px] text-muted-foreground">
                <b className="inline-block w-5 font-semibold text-ink tabular">
                  {Number(day.date.slice(8))}
                </b>{' '}
                {format(parseISO(day.date), 'EEE')}
              </span>
              {day.override === 'suppressed' ? (
                <span className="rounded-[5px] bg-hatch px-[7px] py-px text-[11px] font-[550] text-ink-2 shadow-[inset_0_0_0_1px_#DEDAD1]">
                  Suppressed
                </span>
              ) : (
                <span className="rounded-[5px] bg-brand px-[7px] py-px text-[11px] font-[550] text-white">
                  {day.goal ? formatGoal(day.goal, measure) : 'Custom goal'}
                </span>
              )}
              {onRemove && (
                <Button
                  variant="ghost"
                  size="icon-xs"
                  aria-label={`Remove override for ${formatDay(day.date)}`}
                  disabled={pendingDate === day.date}
                  onClick={() => onRemove(day.date)}
                >
                  <X />
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </RailCard>
  );
}

const LEGEND: { label: string; swatch: string }[] = [
  { label: 'Met', swatch: 'bg-met' },
  { label: 'Missed', swatch: 'bg-[#E58A80]' },
  { label: 'Open today', swatch: 'bg-open' },
  { label: 'Suppressed', swatch: 'bg-hatch shadow-[inset_0_0_0_1px_#DCD8CF]' },
  { label: 'No goal', swatch: 'bg-card shadow-[inset_0_0_0_1px_#DDD9D0]' },
  {
    label: 'Custom goal',
    swatch:
      'bg-brand after:absolute after:inset-0 after:m-auto after:size-1 after:rounded-full after:bg-white',
  },
];

export function Legend() {
  return (
    <ul
      className="flex flex-wrap gap-x-3.5 gap-y-2 px-1 py-0.5 text-[11.5px] text-muted-foreground"
      aria-label="Legend"
    >
      {LEGEND.map((item) => (
        <li key={item.label} className="inline-flex items-center gap-1.5">
          <i
            className={cn('relative inline-block h-2.5 w-3.5 rounded-[2.5px]', item.swatch)}
            aria-hidden
          />
          {item.label}
        </li>
      ))}
    </ul>
  );
}

const SWATCH: Record<EffectKind, string> = {
  start: 'bg-brand',
  gains: 'border-[1.5px] border-dashed border-brand bg-brand-tint',
  suppressed: 'bg-hatch shadow-[inset_0_0_0_1px_#D8D4CB]',
  custom: 'bg-brand',
  earlier: 'bg-card shadow-[inset_0_0_0_1px_#D8D4CB]',
};

function Effects({ effects }: { effects: Effect[] }) {
  return (
    <div className="mt-3.5 flex flex-col gap-[7px] rounded-[9px] bg-background px-3 py-2.5 text-[12.5px] text-ink-2">
      <h4 className="text-[11px] font-semibold tracking-[0.05em] text-faint uppercase">
        When you save
      </h4>
      {effects.map((effect) => (
        <p key={effect.text} className="flex items-start gap-2 leading-[1.4]">
          <i
            className={cn('mt-[5px] h-2.5 w-4 shrink-0 rounded-[3px]', SWATCH[effect.kind])}
            aria-hidden
          />
          {effect.text}
        </p>
      ))}
    </div>
  );
}

function AppliesFrom({
  value,
  today,
  onChange,
  withShortcuts,
}: {
  value: string;
  today: string;
  onChange: (date: string) => void;
  withShortcuts: boolean;
}) {
  const [open, setOpen] = useState(false);
  const weekStart = startOfWeekIso(today);
  const label = value === today ? `Today, ${formatDay(value)}` : formatDayWithYear(value);
  const chip = (on: boolean) =>
    cn(
      'h-[26px] rounded-full border px-2.5 text-xs font-[550] outline-none focus-visible:ring-3 focus-visible:ring-brand/30',
      on ? 'border-brand bg-brand text-white' : 'border-line text-ink-2 hover:bg-line-2'
    );

  return (
    <>
      <FieldLabel>Applies from</FieldLabel>
      <Popover open={open} onOpenChange={setOpen}>
        <PopoverTrigger
          className={cn(
            'flex h-10 w-full items-center gap-2 rounded-lg border bg-card px-3 text-left outline-none',
            open || withShortcuts ? 'border-brand ring-3 ring-brand-tint' : 'border-input'
          )}
        >
          <CalendarDays className="size-4 text-muted-foreground" aria-hidden />
          <span className="flex-1 text-[15px] font-semibold tabular">{label}</span>
          <ChevronDown className="size-4 text-muted-foreground" aria-hidden />
        </PopoverTrigger>
        <PopoverContent align="start" className="w-auto p-1">
          <Calendar
            mode="single"
            weekStartsOn={1}
            selected={parseISO(value)}
            defaultMonth={parseISO(value)}
            onSelect={(date) => {
              if (date) onChange(toIsoDate(date));
              setOpen(false);
            }}
          />
        </PopoverContent>
      </Popover>
      {withShortcuts && (
        <>
          <p className="mt-1.5 flex items-center gap-[5px] text-xs text-muted-foreground">
            <Info className="size-3.5" aria-hidden /> Or click the first day in the calendar.
          </p>
          <div className="mt-2.5 flex flex-wrap gap-1.5">
            <button type="button" className={chip(value === today)} onClick={() => onChange(today)}>
              Today
            </button>
            {weekStart !== today && (
              <button
                type="button"
                className={chip(value === weekStart)}
                onClick={() => onChange(weekStart)}
              >
                Start of this week
              </button>
            )}
            {value !== today && value !== weekStart && (
              <button
                type="button"
                className={chip(true)}
                aria-pressed
                onClick={() => setOpen(true)}
              >
                {formatDayMonth(value)}
              </button>
            )}
          </div>
        </>
      )}
    </>
  );
}

/**
 * The rail's goal form: "Change default goal" for a goal that exists, "Set
 * goal" for a statistic's first one. Reports the goal being typed so the
 * calendar can preview it.
 */
export function GoalFormCard({
  statistic,
  measure,
  current,
  appliesFrom,
  onAppliesFromChange,
  today,
  effects,
  onGoalChange,
  pending,
  error,
  onCancel,
  onSave,
}: {
  statistic: StatisticMetadata;
  measure: Measure;
  current: DefaultGoal | null;
  appliesFrom: string;
  onAppliesFromChange: (date: string) => void;
  today: string;
  effects: Effect[];
  onGoalChange: (goal: GoalData | null) => void;
  pending: boolean;
  error: string | null;
  onCancel: (() => void) | null;
  onSave: (goalData: Record<string, string>) => void;
}) {
  const type = statistic.statistic_type;
  const form = useGoalForm(type, current?.goal ?? null);

  useEffect(() => {
    onGoalChange(formGoal(type, form.getValues()));
    const subscription = form.watch((values) => onGoalChange(formGoal(type, values)));
    return () => subscription.unsubscribe();
  }, [form, type, onGoalChange]);

  const submit = form.handleSubmit((values: FieldValues) => onSave(serializeGoalData(values)));
  const changing = current !== null;

  return (
    <RailCard className="border-brand-line shadow-[0_8px_24px_-12px_rgba(59,76,214,0.35)]">
      <form onSubmit={submit}>
        <div className="mb-2 flex min-h-[26px] items-center justify-between gap-2">
          <h3 className="text-[15px] font-semibold tracking-[-0.01em]">
            {changing ? 'Change default goal' : 'Set goal'}
          </h3>
          {changing && onCancel ? (
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Close"
              onClick={onCancel}
            >
              <X />
            </Button>
          ) : (
            <span className="rounded-full bg-line-2 px-2 py-0.5 text-[11px] font-[550] text-muted-foreground">
              {STATISTIC_TYPES[type].label}
            </span>
          )}
        </div>
        {!changing && (
          <p className="mb-1 text-[12.5px] leading-normal text-muted-foreground">
            Once saved, {statistic.name} joins Active goals.
          </p>
        )}
        <GoalFields form={form} statisticType={type} measure={measure} />
        <AppliesFrom
          value={appliesFrom}
          today={today}
          onChange={onAppliesFromChange}
          withShortcuts={changing}
        />
        {(changing || appliesFrom < today) && effects.length > 0 && <Effects effects={effects} />}
        {error && <p className="mt-2 text-xs text-missed">{error}</p>}
        <div className="mt-4 flex justify-end gap-2">
          {onCancel && (
            <Button type="button" variant="ghost" onClick={onCancel}>
              Cancel
            </Button>
          )}
          <Button type="submit" disabled={pending}>
            {pending ? 'Saving…' : changing ? 'Save default goal' : 'Set goal'}
          </Button>
        </div>
      </form>
    </RailCard>
  );
}

export function NoGoalCard({
  statistic,
  onSetGoal,
}: {
  statistic: StatisticMetadata;
  onSetGoal: (() => void) | null;
}) {
  return (
    <RailCard>
      <CardHeading>Goal</CardHeading>
      <p className="text-[12.5px] leading-normal text-muted-foreground">
        {statistic.name} has no goal yet.
        {!onSetGoal && " Goal editing isn't available for time differences yet."}
      </p>
      {onSetGoal && (
        <Button className="mt-3" size="sm" onClick={onSetGoal}>
          Set goal
        </Button>
      )}
    </RailCard>
  );
}

export function ReadOnlyNote() {
  return (
    <p className="flex items-start gap-1.5 px-1 text-xs leading-normal text-muted-foreground">
      <Info className="mt-px size-3.5 shrink-0" aria-hidden />
      Goal editing isn&apos;t available for time differences yet.
    </p>
  );
}

/** "Nothing logged in the last 30 days. 2 days logged this year, 1 h 39 min in total." */
export function describeActivity(days: Day[], measure: Measure, today: string): string {
  const logged = days.filter((day) => day.value !== null);
  const recentFrom = shiftIsoDate(today, -29);
  const recent = logged.filter((day) => day.date >= recentFrom).length;
  const first =
    recent === 0 ? 'Nothing logged in the last 30 days.' : `${recent} of the last 30 days logged.`;
  if (logged.length === 0) return `${first} Nothing logged this year.`;
  const total = logged.reduce(
    (sum, day) => sum + (typeof day.value === 'number' ? day.value : 0),
    0
  );
  const totalText =
    measure.kind === 'duration' || measure.kind === 'spans'
      ? `, ${formatSeconds(total)} in total`
      : measure.kind === 'count'
        ? `, ${formatCount(total)} in total`
        : '';
  return `${first} ${logged.length} ${logged.length === 1 ? 'day' : 'days'} logged this year${totalText}.`;
}

export function RecentActivityCard({
  days,
  measure,
  today,
}: {
  days: Day[] | undefined;
  measure: Measure;
  today: string;
}) {
  return (
    <RailCard>
      <CardHeading>Recent activity</CardHeading>
      <p className="text-[12.5px] leading-normal text-muted-foreground">
        {days ? describeActivity(days, measure, today) : 'Loading…'}
      </p>
    </RailCard>
  );
}
