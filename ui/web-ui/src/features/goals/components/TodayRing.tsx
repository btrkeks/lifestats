import { Check } from 'lucide-react';
import type { Day } from '../day';

const SIZE = 30;
const RADIUS = 12.5;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

/** Today's progress at a glance: an arc, a check when met, dashes when there's nothing to do. */
export function TodayRing({ day }: { day: Day | undefined }) {
  if (day?.status === 'met') {
    return (
      <span
        className="grid size-[30px] shrink-0 place-items-center rounded-full bg-met text-white"
        aria-hidden
      >
        <Check className="size-4" strokeWidth={2.5} />
      </span>
    );
  }
  if (!day || day.status === 'suppressed' || day.status === 'none') {
    return (
      <svg width={SIZE} height={SIZE} className="shrink-0" aria-hidden>
        <circle
          cx={SIZE / 2}
          cy={SIZE / 2}
          r={RADIUS}
          fill="none"
          stroke="#CFCBC2"
          strokeWidth={2.5}
          strokeDasharray="2 3.2"
        />
      </svg>
    );
  }
  const percent = Math.min(100, day.percentage_achieved ?? 0);
  return (
    <span className="relative grid size-[30px] shrink-0 place-items-center" aria-hidden>
      <svg width={SIZE} height={SIZE} className="absolute inset-0 -rotate-90">
        <circle
          cx={SIZE / 2}
          cy={SIZE / 2}
          r={RADIUS}
          fill="none"
          stroke="var(--line-2)"
          strokeWidth={3}
        />
        <circle
          cx={SIZE / 2}
          cy={SIZE / 2}
          r={RADIUS}
          fill="none"
          stroke={day.status === 'missed' ? 'var(--missed)' : 'var(--open)'}
          strokeWidth={3}
          strokeLinecap="round"
          strokeDasharray={`${(percent / 100) * CIRCUMFERENCE} ${CIRCUMFERENCE}`}
        />
      </svg>
      <b className="relative text-[9.5px] font-[650] text-ink-2 tabular">{percent}</b>
    </span>
  );
}
