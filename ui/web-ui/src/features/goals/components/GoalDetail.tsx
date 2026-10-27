'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import { format, parseISO } from 'date-fns';
import { Flag } from 'lucide-react';
import { errorMessage } from '@/lib/api';
import { shiftIsoDate } from '@/lib/dates';
import { measureOf } from '@/lib/measure';
import { createSuppressedRequest } from '@/lib/schemas';
import { canEditGoals } from '@/lib/statisticTypes';
import type { GoalData, StatisticMetadata } from '@/types';
import { datesBetween, monthGrid, monthOf } from '@/features/calendar/grid';
import type { Day } from '../day';
import { describeEffects, previewMark } from '../defaultGoalPreview';
import {
  useDefaultGoal,
  useDays,
  useDeleteDailyGoal,
  useSetDailyGoal,
  useSetDailyGoalRange,
  useSetDefaultGoal,
  useStreaks,
} from '../hooks/useGoals';
import { goalLevel, metCount, trendPoints, trendReadout } from '../insights';
import { todayFigure } from '../today';
import { DayActions } from './DayActions';
import { DetailHeader, type Figure } from './DetailHeader';
import { MonthCalendar, type CalendarMode } from './MonthCalendar';
import {
  DefaultGoalCard,
  GoalFormCard,
  Legend,
  NoGoalCard,
  OverridesCard,
  ReadOnlyNote,
  RecentActivityCard,
} from './Rail';
import { SelectionBar } from './SelectionBar';

/** What the calendar and rail are doing right now. */
type Interaction =
  | { kind: 'idle' }
  | { kind: 'day'; date: string } // day popover open
  | { kind: 'range'; anchor: string; end: string } // docked selection bar
  | { kind: 'goalForm'; appliesFrom: string }; // rail form; calendar clicks pick the start

const TREND_DAYS = 84;

function plural(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? '' : 's'}`;
}

/** The selected statistic: header with trend, month calendar and the goal rail. */
export function GoalDetail({ statistic, today }: { statistic: StatisticMetadata; today: string }) {
  const measure = measureOf(statistic);
  const editable = canEditGoals(statistic.statistic_type);
  const [month, setMonth] = useState(monthOf(today));
  const [interaction, setInteraction] = useState<Interaction>(() =>
    statistic.default_goal === null && editable
      ? { kind: 'goalForm', appliesFrom: today }
      : { kind: 'idle' }
  );
  const [draftGoal, setDraftGoal] = useState<GoalData | null>(null);

  const defaultGoal = useDefaultGoal(statistic);
  const current = defaultGoal.data ?? null;
  const hasGoal = statistic.default_goal !== null;
  const grid = monthGrid(month);
  const monthDays = useDays(statistic, { start: grid.start, end: grid.end, today });
  const trendDays = useDays(statistic, {
    start: shiftIsoDate(today, -(TREND_DAYS - 1)),
    end: today,
    today,
  });
  const formStart = interaction.kind === 'goalForm' ? interaction.appliesFrom : today;
  const previewDays = useDays(statistic, {
    start: formStart,
    end: today,
    today,
    enabled: interaction.kind === 'goalForm',
  });
  const activityDays = useDays(statistic, {
    start: `${today.slice(0, 4)}-01-01`,
    end: today,
    today,
    enabled: !hasGoal,
  });
  const [streak] = useStreaks([statistic.id]);

  const setDefault = useSetDefaultGoal(statistic.id);
  const setDaily = useSetDailyGoal(statistic.id);
  const setRange = useSetDailyGoalRange(statistic.id);
  const deleteDaily = useDeleteDailyGoal(statistic.id);
  const backToIdle = () => setInteraction({ kind: 'idle' });

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && interaction.kind === 'range') setInteraction({ kind: 'idle' });
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [interaction.kind]);

  const changeMonth = (next: string) => {
    setMonth(next);
    rangeAnchor.current = null;
    if (interaction.kind === 'day' || interaction.kind === 'range') backToIdle();
  };

  // The day Shift-click extends from. A ref, because the popover may already
  // have closed (outside press) by the time the Shift-click arrives.
  const rangeAnchor = useRef<string | null>(null);

  const onDayClick = (date: string, extend: boolean) => {
    setDaily.reset();
    deleteDaily.reset();
    const anchor = rangeAnchor.current;
    if (extend && anchor && anchor !== date) {
      setRange.reset();
      setInteraction({ kind: 'range', anchor, end: date });
    } else {
      rangeAnchor.current = date;
      setInteraction({ kind: 'day', date });
    }
  };

  const onDrag = (anchor: string, end: string) => {
    setRange.reset();
    rangeAnchor.current = anchor;
    setInteraction(anchor === end ? { kind: 'day', date: anchor } : { kind: 'range', anchor, end });
  };

  const onDraftGoal = useCallback((goal: GoalData | null) => setDraftGoal(goal), []);

  // ---- derived view state ----
  const days = monthDays.data;
  const monthOnly = (days ?? []).filter((day) => day.date.startsWith(month));
  const todayDay = trendDays.data?.find((day) => day.date === today);
  const monthName = format(parseISO(`${month}-01`), 'MMMM');

  const figures: Figure[] = hasGoal
    ? [
        (() => {
          const { met, withGoal } = metCount(monthOnly.filter((day) => day.date <= today));
          return { label: monthName, value: `${met} of ${withGoal} met` };
        })(),
        { label: 'Streak', value: streak?.data === undefined ? '–' : plural(streak.data, 'day') },
        { label: 'Today', value: todayDay ? todayFigure(todayDay, measure) : '–' },
      ]
    : [{ label: 'Goal', value: 'none yet' }];

  const trend =
    hasGoal && current && trendDays.data
      ? {
          points: trendPoints(trendDays.data, measure, goalLevel(current.goal)),
          level: goalLevel(current.goal),
          readout: trendReadout(trendDays.data, measure, current.goal),
        }
      : null;

  const selection =
    interaction.kind === 'range'
      ? interaction.anchor <= interaction.end
        ? { from: interaction.anchor, to: interaction.end }
        : { from: interaction.end, to: interaction.anchor }
      : null;

  const mode: CalendarMode = !editable
    ? 'inert'
    : interaction.kind === 'goalForm'
      ? 'pick'
      : hasGoal
        ? 'act'
        : 'inert';

  const preview = (day: Day) => {
    if (interaction.kind !== 'goalForm') return null;
    const mark = previewMark(day, interaction.appliesFrom, draftGoal);
    if (mark === 'before' && !hasGoal) return null;
    return { mark, goal: draftGoal, isStart: day.date === interaction.appliesFrom };
  };

  const hint =
    mode === 'pick' && hasGoal ? (
      <span className="inline-flex items-center gap-1.5 rounded-full bg-brand-tint py-1 pr-[11px] pl-[9px] text-xs font-semibold text-brand">
        <Flag className="size-3.5" aria-hidden /> Picking the start day · click a day
      </span>
    ) : !hasGoal ? (
      'No goal yet · days show here once one is set'
    ) : mode === 'inert' ? (
      "Goals for time differences can't be edited yet"
    ) : selection ? (
      'Shift-click or drag to change the selection · Esc to clear'
    ) : (
      'Click a day to change it · Shift-click or drag to select a range'
    );

  const targetDay =
    interaction.kind === 'day' ? days?.find((day) => day.date === interaction.date) : undefined;
  const dayError = setDaily.error ?? deleteDaily.error;

  const popover = targetDay ? (
    <DayActions
      key={targetDay.date}
      day={targetDay}
      measure={measure}
      statisticType={statistic.statistic_type}
      defaultGoal={current?.goal ?? null}
      pending={setDaily.isPending || deleteDaily.isPending}
      error={dayError ? errorMessage(dayError) : null}
      onSuppress={() =>
        setDaily.mutate(
          { date: targetDay.date, request: createSuppressedRequest() },
          { onSuccess: backToIdle }
        )
      }
      onRemoveOverride={() => deleteDaily.mutate(targetDay.date, { onSuccess: backToIdle })}
      onSetGoal={(goalData) =>
        setDaily.mutate(
          { date: targetDay.date, request: { kind: 'goal', goal_data: goalData } },
          { onSuccess: backToIdle }
        )
      }
      onClose={backToIdle}
    />
  ) : null;

  const footer =
    selection && days ? (
      <SelectionBar
        from={selection.from}
        to={selection.to}
        days={days.filter((day) => datesBetween(selection.from, selection.to).includes(day.date))}
        measure={measure}
        statisticType={statistic.statistic_type}
        defaultGoal={current?.goal ?? null}
        pending={setRange.isPending}
        error={setRange.error ? errorMessage(setRange.error) : null}
        onSetGoal={(goalData) =>
          setRange.mutate(
            { start: selection.from, end: selection.to, kind: 'goal', goal_data: goalData },
            { onSuccess: backToIdle }
          )
        }
        onSuppress={() =>
          setRange.mutate(
            { start: selection.from, end: selection.to, kind: 'suppressed' },
            { onSuccess: backToIdle }
          )
        }
        onClear={backToIdle}
      />
    ) : null;

  const effects =
    interaction.kind === 'goalForm'
      ? describeEffects({
          days: previewDays.data ?? [],
          appliesFrom: interaction.appliesFrom,
          newGoal: draftGoal,
          current,
          measure,
          today,
        })
      : [];

  const rail = (() => {
    if (defaultGoal.isPending) return null;
    if (interaction.kind === 'goalForm') {
      return (
        <>
          <GoalFormCard
            statistic={statistic}
            measure={measure}
            current={current}
            appliesFrom={interaction.appliesFrom}
            onAppliesFromChange={(date) => setInteraction({ kind: 'goalForm', appliesFrom: date })}
            today={today}
            effects={effects}
            onGoalChange={onDraftGoal}
            pending={setDefault.isPending}
            error={setDefault.error ? errorMessage(setDefault.error) : null}
            onCancel={backToIdle}
            onSave={(goalData) =>
              setDefault.mutate(
                { goal_data: goalData, valid_since: interaction.appliesFrom },
                { onSuccess: backToIdle }
              )
            }
          />
          {!hasGoal && (
            <RecentActivityCard days={activityDays.data} measure={measure} today={today} />
          )}
        </>
      );
    }
    if (!current) {
      return (
        <>
          <NoGoalCard
            statistic={statistic}
            onSetGoal={
              editable ? () => setInteraction({ kind: 'goalForm', appliesFrom: today }) : null
            }
          />
          <RecentActivityCard days={activityDays.data} measure={measure} today={today} />
        </>
      );
    }
    return (
      <>
        <DefaultGoalCard
          current={current}
          measure={measure}
          onChange={
            editable
              ? () => {
                  setDefault.reset();
                  setInteraction({ kind: 'goalForm', appliesFrom: today });
                }
              : null
          }
        />
        {!editable && <ReadOnlyNote />}
        <OverridesCard
          days={monthOnly}
          month={monthName}
          measure={measure}
          pendingDate={deleteDaily.isPending ? (deleteDaily.variables ?? null) : null}
          onRemove={editable ? (date) => deleteDaily.mutate(date) : null}
        />
        <Legend />
      </>
    );
  })();

  return (
    <div className="flex min-h-full flex-col pt-5 pr-6 pb-6 pl-7">
      <DetailHeader
        statistic={statistic}
        measure={measure}
        hasGoal={hasGoal}
        trend={trend}
        figures={figures}
      />
      <div className="flex gap-6 pt-3.5">
        <div className="min-w-[540px] flex-1">
          <MonthCalendar
            month={month}
            onMonthChange={changeMonth}
            today={today}
            days={days}
            measure={measure}
            quiet={!hasGoal}
            mode={mode}
            hint={hint}
            selection={selection}
            target={interaction.kind === 'day' ? interaction.date : null}
            preview={preview}
            onPick={(date) => setInteraction({ kind: 'goalForm', appliesFrom: date })}
            onDayClick={onDayClick}
            onDrag={onDrag}
            popover={popover}
            onPopoverClose={backToIdle}
            footer={footer}
          />
          {monthDays.isError && (
            <p className="mt-2 text-xs text-missed">{errorMessage(monthDays.error)}</p>
          )}
        </div>
        <aside className="flex w-[320px] shrink-0 flex-col gap-3 pt-[42px]" aria-label="Goal">
          {rail}
        </aside>
      </div>
    </div>
  );
}
