'use client';

import { forwardRef, type KeyboardEvent, type MouseEvent, type PointerEvent } from 'react';
import { Flag } from 'lucide-react';
import { cn } from 'cn';
import { formatDayLong, formatShortMonth } from '@/lib/dates';
import { formatClock, formatGoal } from '@/lib/goalFormat';
import { formatValue, type Measure } from '@/lib/measure';
import type { GoalData } from '@/types';
import { DAY_STATUS_LABEL, type Day, type DayStatus } from '../day';
import type { PreviewMark } from '../defaultGoalPreview';

const CELL: Record<DayStatus, string> = {
  met: 'bg-met-tint border-[#CDE8D9]',
  missed: 'bg-missed-tint border-[#F4D2CD]',
  open: 'bg-open-tint border-[#F0D9AE]',
  suppressed: 'bg-hatch border-[#E2DFD8]',
  none: 'bg-card border-dashed border-[#D9D5CC]',
  future: 'bg-transparent border-dashed border-[#E2DFD8]',
};

const VALUE: Partial<Record<DayStatus, string>> = {
  met: 'text-[#14643f]',
  missed: 'text-[#a8352c]',
  open: 'text-[#8a5a0c]',
};

const SUB: Partial<Record<DayStatus, string>> = {
  met: 'text-[#3e8a64]',
  missed: 'text-[#c0574d]',
  open: 'text-[#a6731f]',
  suppressed: 'text-muted-foreground',
  none: 'text-faint',
};

const BAR: Partial<Record<DayStatus, string>> = {
  met: 'bg-met',
  missed: 'bg-missed',
  open: 'bg-open',
};

/** The big number and the line under it. */
function cellBody(day: Day, measure: Measure): { main: string | null; sub: string | null } {
  const value = day.value === null ? null : formatValue(measure, day.value);
  switch (day.status) {
    case 'met':
    case 'missed':
    case 'open': {
      const percent = `${day.percentage_achieved ?? 0}%`;
      if (value !== null) return { main: value, sub: percent };
      if (day.goal?.type === 'PointInTime') {
        return day.status === 'open'
          ? { main: 'Due', sub: `by ${formatClock(day.goal.data.deadline)}` }
          : { main: '—', sub: percent };
      }
      return { main: measure.kind === 'count' ? '0' : '—', sub: percent };
    }
    case 'suppressed':
      return { main: value, sub: 'Suppressed' };
    case 'none':
      return { main: value, sub: 'No goal' };
    case 'future':
      return { main: null, sub: null };
  }
}

export interface DayCellProps {
  date: string;
  /** null while loading */
  day: Day | null;
  measure: Measure;
  today: string;
  /** "YYYY-MM" being shown; other months' days get their month name */
  month: string;
  /** The statistic has never had a goal: plain cells, no "No goal" labels */
  quiet: boolean;
  interactive: boolean;
  selected: boolean;
  /** The day popover is open on this cell */
  targeted: boolean;
  preview: { mark: PreviewMark; goal: GoalData | null; isStart: boolean } | null;
  onPointerDown?: (event: PointerEvent<HTMLButtonElement>) => void;
  onPointerEnter?: (event: PointerEvent<HTMLButtonElement>) => void;
  onClick?: (event: MouseEvent<HTMLButtonElement>) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLButtonElement>) => void;
}

export const DayCell = forwardRef<HTMLButtonElement, DayCellProps>(function DayCell(
  {
    date,
    day,
    measure,
    today,
    month,
    quiet,
    interactive,
    selected,
    targeted,
    preview,
    onPointerDown,
    onPointerEnter,
    onClick,
    onKeyDown,
  },
  ref
) {
  const status: DayStatus = day?.status ?? (date > today ? 'future' : 'none');
  const dayOfMonth = Number(date.slice(8, 10));
  const showMonth = dayOfMonth === 1 || date.slice(0, 7) !== month;
  const isToday = date === today;
  const gains = preview?.mark === 'gains';
  const body = day ? cellBody(day, measure) : { main: null, sub: null };
  const sub = gains ? 'New goal' : quiet && status === 'none' ? null : body.sub;
  const customGoal = day?.override === 'goal' && day.goal ? formatGoal(day.goal, measure) : null;
  const showBar = !gains && (status === 'met' || status === 'missed' || status === 'open');
  const percent = Math.min(100, day?.percentage_achieved ?? 0);

  const label = [
    formatDayLong(date),
    isToday ? 'today' : null,
    day ? DAY_STATUS_LABEL[status] : 'loading',
    body.main && status !== 'future' ? body.main : null,
    customGoal ? `custom goal ${customGoal}` : null,
  ]
    .filter(Boolean)
    .join(', ');

  return (
    <button
      ref={ref}
      type="button"
      data-date={date}
      aria-label={label}
      aria-pressed={selected || undefined}
      disabled={!interactive}
      onPointerDown={onPointerDown}
      onPointerEnter={onPointerEnter}
      onClick={onClick}
      onKeyDown={onKeyDown}
      className={cn(
        'relative flex min-h-[112px] flex-col overflow-hidden rounded-[10px] border px-[9px] pt-2 pb-[9px] text-left outline-none select-none',
        'focus-visible:ring-3 focus-visible:ring-brand/40 disabled:cursor-default',
        quiet && status === 'none' ? 'border-line-2 bg-card' : CELL[status],
        gains && 'border-[1.5px] border-dashed border-brand bg-brand-tint',
        selected &&
          !targeted &&
          'border-2 border-solid border-brand bg-brand-tint px-2 pt-[7px] pb-2',
        targeted &&
          'border-2 border-solid border-brand px-2 pt-[7px] pb-2 shadow-[0_0_0_4px_rgba(59,76,214,0.18)]',
        isToday && !selected && !targeted && 'shadow-[inset_0_0_0_2px_var(--ink)]',
        preview?.mark === 'before' && 'opacity-45',
        interactive && 'hover:brightness-[0.985]'
      )}
    >
      <span className="flex min-h-[18px] items-center gap-[5px]">
        <span
          className={cn(
            'text-[13px] font-[620] tabular',
            status === 'future'
              ? 'text-faint'
              : date.slice(0, 7) !== month
                ? 'text-muted-foreground'
                : 'text-ink'
          )}
        >
          {dayOfMonth}
          {showMonth && (
            <small className="ml-1 text-[10.5px] font-medium text-muted-foreground">
              {formatShortMonth(date)}
            </small>
          )}
        </span>
        {isToday && (
          <span className="rounded-[4px] bg-ink px-[5px] text-[10px] leading-4 font-[650] text-white">
            Today
          </span>
        )}
        {preview?.isStart && !isToday ? (
          <span className="ml-auto inline-flex items-center gap-[3px] rounded-[4px] bg-brand px-[5px] text-[10px] leading-4 font-semibold text-white">
            <Flag className="size-2.5" aria-hidden /> Start
          </span>
        ) : gains && preview.goal && !isToday ? (
          <span className="ml-auto shrink-0 rounded-[4px] bg-card px-1 whitespace-nowrap text-[10px] leading-4 font-semibold text-brand shadow-[inset_0_0_0_1px_var(--brand-line)]">
            {formatGoal(preview.goal, measure)}
          </span>
        ) : (
          customGoal && (
            <span className="ml-auto shrink-0 rounded-[4px] bg-brand px-1 whitespace-nowrap text-[10px] leading-4 font-semibold text-white">
              {customGoal}
            </span>
          )
        )}
      </span>
      <span className={cn('mt-auto flex flex-col', showBar && 'mb-[5px]')}>
        {body.main !== null && (
          <span
            className={cn(
              'text-[19px] leading-[1.15] font-[620] tracking-[-0.01em] tabular',
              VALUE[status] ?? 'text-ink-2',
              selected && !targeted && 'text-[#2d3aa8]'
            )}
          >
            {body.main}
          </span>
        )}
        {sub && (
          <span
            className={cn(
              'text-[11px] font-medium',
              gains || (selected && !targeted) ? 'text-[#5562c9]' : SUB[status]
            )}
          >
            {sub}
          </span>
        )}
      </span>
      {showBar && (
        <span
          className="absolute right-[9px] bottom-1.5 left-[9px] h-[3px] rounded-[2px] bg-black/6"
          aria-hidden
        >
          <i
            className={cn('block h-full rounded-[2px]', BAR[status])}
            style={{ width: `${percent}%` }}
          />
        </span>
      )}
    </button>
  );
});
