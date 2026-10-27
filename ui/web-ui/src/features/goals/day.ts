import type { AchievementDay } from '@/types';

/**
 * How a calendar day reads. `open` is today while its goal isn't met yet;
 * `future` days carry no state yet.
 */
export type DayStatus = 'met' | 'missed' | 'open' | 'suppressed' | 'none' | 'future';

export interface Day extends AchievementDay {
  status: DayStatus;
}

export const DAY_STATUS_LABEL: Record<DayStatus, string> = {
  met: 'Met',
  missed: 'Missed',
  open: 'Open today',
  suppressed: 'Suppressed',
  none: 'No goal',
  future: 'Upcoming',
};

/** Classifies one day. `today` is YYYY-MM-DD (ISO dates compare as strings). */
export function classifyDay(day: AchievementDay, today: string): DayStatus {
  if (day.date > today) return 'future';
  if (day.override === 'suppressed') return 'suppressed';
  if (!day.has_goal) return 'none';
  if (day.achieved) return 'met';
  return day.date === today ? 'open' : 'missed';
}

export function toDays(days: AchievementDay[], today: string): Day[] {
  return days.map((day) => ({ ...day, status: classifyDay(day, today) }));
}
