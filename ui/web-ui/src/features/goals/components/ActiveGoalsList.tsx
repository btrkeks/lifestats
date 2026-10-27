'use client';

import { ChevronRight, Flame, List } from 'lucide-react';
import { cn } from 'cn';
import type { StatisticMetadata } from '@/types';
import { formatGoal } from '@/lib/goalFormat';
import { measureOf } from '@/lib/measure';
import { parseStatisticDefaultGoal } from '@/lib/schemas';
import type { Day } from '../day';
import { todayCounts, todayProgress, type Tone } from '../today';
import { TodayRing } from './TodayRing';

export const TONE_CLASS: Record<Tone, string> = {
  met: 'text-met',
  open: 'text-[#a86c12]',
  missed: 'text-missed',
  muted: 'text-muted-foreground',
};

export interface ActiveGoal {
  statistic: StatisticMetadata;
  today: Day | undefined;
  streak: number | undefined;
}

function GoalRow({
  goal,
  selected,
  divider,
  onSelect,
}: {
  goal: ActiveGoal;
  selected: boolean;
  /** Hairline above the row (hidden around the selected row) */
  divider: boolean;
  onSelect: () => void;
}) {
  const { statistic, today, streak } = goal;
  const measure = measureOf(statistic);
  const defaultGoal = parseStatisticDefaultGoal(statistic);
  const progress = today ? todayProgress(today, measure) : null;

  return (
    <li className="relative">
      <button
        type="button"
        onClick={onSelect}
        aria-current={selected ? 'true' : undefined}
        className={cn(
          'relative flex w-full items-center gap-3 rounded-[10px] px-2.5 py-[15px] text-left outline-none transition-colors',
          'focus-visible:ring-3 focus-visible:ring-brand/30',
          selected ? 'bg-brand-tint' : 'hover:bg-[#f7f6f3]'
        )}
      >
        {selected && (
          <span
            className="absolute top-2.5 bottom-2.5 -left-2 w-[3px] rounded-r-[3px] bg-brand"
            aria-hidden
          />
        )}
        <TodayRing day={today} />
        <span className="min-w-0 flex-1">
          <span className="flex items-baseline gap-2">
            <span className="min-w-0 flex-1 truncate text-[13.5px] font-[560] tracking-[-0.005em]">
              {statistic.name}
            </span>
            {defaultGoal && (
              <span className="shrink-0 text-xs font-[550] text-ink-2 tabular">
                {formatGoal(defaultGoal, measure)}
              </span>
            )}
          </span>
          <span className="mt-[3px] flex items-center gap-2 text-[11.5px]">
            {progress ? (
              <span className={TONE_CLASS[progress.tone]}>{progress.text}</span>
            ) : (
              <span className="text-faint">Loading…</span>
            )}
            {streak !== undefined && streak > 0 && (
              <span
                className="inline-flex items-center gap-0.5 font-[550] text-[#b8542a]"
                title={`${streak}-day streak`}
              >
                <Flame className="size-3" aria-hidden />
                {streak} d
              </span>
            )}
          </span>
        </span>
      </button>
      {divider && (
        <span
          className="pointer-events-none absolute -top-px right-2.5 left-[52px] h-px bg-line-2"
          aria-hidden
        />
      )}
    </li>
  );
}

export function ActiveGoalsList({
  goals,
  selectedId,
  onSelect,
  withoutGoalCount,
  onShowWithoutGoal,
}: {
  goals: ActiveGoal[];
  selectedId: number | null;
  onSelect: (id: number) => void;
  withoutGoalCount: number;
  onShowWithoutGoal: () => void;
}) {
  const counts = todayCounts(goals.flatMap((goal) => (goal.today ? [goal.today] : [])));
  const summary = [
    { count: counts.met, label: 'met', className: 'text-met' },
    { count: counts.open, label: 'open', className: 'text-open' },
    { count: counts.missed, label: 'missed', className: 'text-missed' },
    { count: counts.suppressed, label: 'suppressed', className: 'text-ink' },
  ].filter((part) => part.count > 0);
  const selectedIndex = goals.findIndex((goal) => goal.statistic.id === selectedId);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <p className="flex items-baseline gap-2 px-[18px] pt-3 pb-2 text-xs text-muted-foreground">
        <span className="text-[12.5px] font-semibold text-ink">Today</span>
        <span>
          {summary.map((part, i) => (
            <span key={part.label}>
              {i > 0 && ' · '}
              <b className={cn('font-semibold', part.className)}>{part.count}</b> {part.label}
            </span>
          ))}
        </span>
      </p>
      <ul className="flex-1 overflow-y-auto px-2">
        {goals.map((goal, index) => (
          <GoalRow
            key={goal.statistic.id}
            goal={goal}
            selected={goal.statistic.id === selectedId}
            divider={index > 0 && index !== selectedIndex && index !== selectedIndex + 1}
            onSelect={() => onSelect(goal.statistic.id)}
          />
        ))}
      </ul>
      {withoutGoalCount > 0 && (
        <button
          type="button"
          onClick={onShowWithoutGoal}
          className="mx-3.5 mt-2 mb-3.5 flex h-10 items-center gap-2.5 rounded-[10px] border border-dashed border-[#D6D2C8] px-3 text-left font-[550] text-ink-2 hover:bg-[#f7f6f3]"
        >
          <List className="size-4 text-muted-foreground" aria-hidden />
          <span className="flex-1">{withoutGoalCount} statistics without a goal</span>
          <ChevronRight className="size-4 text-muted-foreground" aria-hidden />
        </button>
      )}
    </div>
  );
}
