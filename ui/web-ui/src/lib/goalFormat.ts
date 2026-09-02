import type { GoalData } from '@/types';

/** "07:45:00" → "07:45"; seconds are kept only when non-zero. */
export function formatClock(time: string): string {
  const [hours, minutes, seconds] = time.split(':');
  if (hours === undefined || minutes === undefined) {
    return time;
  }
  return seconds && seconds !== '00' ? `${hours}:${minutes}:${seconds}` : `${hours}:${minutes}`;
}

/** 5400 → "1h 30m"; 45 → "45s". */
export function formatDuration(totalSeconds: number): string {
  const sign = totalSeconds < 0 ? '-' : '';
  const abs = Math.abs(totalSeconds);
  const hours = Math.floor(abs / 3600);
  const minutes = Math.floor((abs % 3600) / 60);
  const seconds = abs % 60;

  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (seconds > 0 || parts.length === 0) parts.push(`${seconds}s`);
  return sign + parts.join(' ');
}

/** One-line, human-readable summary of a goal, e.g. "≥ 27,000" or "by 22:00". */
export function formatGoal(goal: GoalData): string {
  switch (goal.type) {
    case 'IntegerAmount':
    case 'Summing':
      return `${goal.data.is_at_least ? '≥' : '≤'} ${goal.data.target.toLocaleString('en-US')}`;
    case 'TimeDifference':
      return `${goal.data.is_at_least ? '≥' : '≤'} ${formatDuration(goal.data.target)}`;
    case 'Habit':
      return goal.data.occurred ? 'Done' : 'Not done';
    case 'PointInTime':
      return `by ${formatClock(goal.data.deadline)}`;
    case 'TimeSpans': {
      const frames = goal.data.target
        .map((frame) => `${formatClock(frame.start)}–${formatClock(frame.end)}`)
        .join(', ');
      return `${frames} · ${goal.data.efficiency}% eff.`;
    }
  }
}
