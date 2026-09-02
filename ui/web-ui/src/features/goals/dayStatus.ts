import type { DailyAchievement } from '@/types';

export type DayStatus = 'achieved' | 'missed' | 'pending' | 'suppressed' | 'no_goal';

export const DAY_STATUS_LABEL: Record<DayStatus, string> = {
  achieved: 'Achieved',
  missed: 'Missed',
  pending: 'In progress',
  suppressed: 'Suppressed',
  no_goal: 'No goal',
};

/** Classifies one day of the achievement strip. `today` is YYYY-MM-DD. */
export function classifyDay(day: DailyAchievement, today: string): DayStatus {
  if (day.override === 'suppressed') return 'suppressed';
  if (!day.has_goal) return 'no_goal';
  if (day.achieved) return 'achieved';
  return day.date === today ? 'pending' : 'missed';
}
