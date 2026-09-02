'use client';

import { useState } from 'react';
import type { DailyAchievement } from '@/types';
import { formatDayLabel, shiftIsoDate, weekdayInitial } from '@/lib/dates';
import { useDailyAchievements } from '../hooks/useGoals';
import { classifyDay, DAY_STATUS_LABEL, type DayStatus } from '../dayStatus';
import { DayPopover } from './DayPopover';

export const STRIP_DAYS = 14;

export const DAY_CELL_CLASSES: Record<DayStatus, string> = {
  achieved: 'bg-emerald-500',
  missed: 'bg-rose-400',
  pending: 'bg-amber-300',
  suppressed:
    'border border-gray-300 bg-[repeating-linear-gradient(135deg,#e5e7eb_0_3px,#f9fafb_3px_6px)]',
  no_goal: 'border border-gray-200 bg-gray-100',
};

interface DayStripProps {
  statisticId: number;
  today: string;
  onEditDay: (date: string) => void;
}

function tooltip(day: DailyAchievement, status: DayStatus): string {
  const parts = [formatDayLabel(day.date), DAY_STATUS_LABEL[status]];
  if (day.percentage_achieved !== null) parts.push(`${day.percentage_achieved}%`);
  if (day.override === 'goal') parts.push('custom goal');
  return parts.join(' · ');
}

/** The last `STRIP_DAYS` days as clickable cells; clicking opens the day's actions. */
export function DayStrip({ statisticId, today, onEditDay }: DayStripProps) {
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const start = shiftIsoDate(today, -(STRIP_DAYS - 1));
  const { data, isLoading, isError } = useDailyAchievements(statisticId, start, today);

  if (isLoading) {
    return <div className="h-12 animate-pulse rounded bg-gray-100" aria-label="Loading days" />;
  }
  if (isError || !data) {
    return <p className="text-sm text-rose-600">Couldn’t load recent days.</p>;
  }

  return (
    <div className="flex gap-1">
      {data.days.map((day, index) => {
        const status = classifyDay(day, today);
        const isSelected = selectedDate === day.date;
        const isToday = day.date === today;
        return (
          <div key={day.date} className="relative flex flex-1 flex-col items-center gap-1">
            <span className="text-[10px] font-medium uppercase text-gray-400">
              {weekdayInitial(day.date)}
            </span>
            <button
              type="button"
              onClick={() => setSelectedDate(isSelected ? null : day.date)}
              title={tooltip(day, status)}
              aria-label={tooltip(day, status)}
              aria-expanded={isSelected}
              className={`flex h-7 w-full items-center justify-center rounded transition hover:opacity-80 ${
                DAY_CELL_CLASSES[status]
              } ${isToday ? 'ring-2 ring-gray-900 ring-offset-1' : ''} ${
                isSelected ? 'outline-2 outline-offset-2 outline-blue-500' : ''
              }`}
            >
              {day.override === 'goal' && (
                <span className="h-1.5 w-1.5 rounded-full bg-white/90 shadow" aria-hidden="true" />
              )}
            </button>
            {isSelected && (
              <DayPopover
                statisticId={statisticId}
                day={day}
                status={status}
                align={index < STRIP_DAYS / 2 ? 'left' : 'right'}
                onClose={() => setSelectedDate(null)}
                onEditGoal={() => {
                  setSelectedDate(null);
                  onEditDay(day.date);
                }}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
