import { format, parseISO } from 'date-fns';
import type { Day } from './day';

/** "Mon 12 – Wed 21 Oct", "Wed 30 Sep – Fri 2 Oct" */
export function formatSelectionSpan(from: string, to: string): string {
  const start = parseISO(from);
  const end = parseISO(to);
  const sameMonth = from.slice(0, 7) === to.slice(0, 7);
  return `${format(start, sameMonth ? 'EEE d' : 'EEE d MMM')} – ${format(end, 'EEE d MMM')}`;
}

const STATE_WORDS: [(day: Day) => boolean, string][] = [
  [(day) => day.override === 'goal', 'with a custom goal'],
  [(day) => day.status === 'suppressed', 'suppressed'],
  [(day) => day.status === 'none', 'without a goal'],
  [(day) => day.status === 'met', 'met'],
  [(day) => day.status === 'missed', 'missed'],
  [(day) => day.status === 'open', 'open'],
];

/** "all without a goal", "7 missed, 3 suppressed" (each day counted once) */
export function describeSelection(days: Day[]): string {
  const counts = new Map<string, number>();
  for (const day of days) {
    const word = STATE_WORDS.find(([matches]) => matches(day))?.[1];
    if (word) counts.set(word, (counts.get(word) ?? 0) + 1);
  }
  const parts = STATE_WORDS.flatMap(([, word]) => {
    const count = counts.get(word);
    return count ? [{ word, count }] : [];
  });
  if (parts.length === 1 && parts[0].count === days.length) return `all ${parts[0].word}`;
  return parts
    .slice(0, 3)
    .map((part) => `${part.count} ${part.word}`)
    .join(', ');
}
