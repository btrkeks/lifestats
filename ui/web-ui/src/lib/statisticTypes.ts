import {
  ChartNoAxesColumn,
  CircleCheck,
  Clock,
  Coffee,
  Hourglass,
  Sigma,
  SquareSplitHorizontal,
  type LucideIcon,
} from 'lucide-react';
import type { StatisticType } from '@/types';

export interface StatisticTypeInfo {
  type: StatisticType;
  /** Short label for meta lines, e.g. "Yomikata · Count · #134" */
  label: string;
  title: string;
  description: string;
  icon: LucideIcon;
  origin: 'logged' | 'derived' | 'source';
}

export const STATISTIC_TYPES: Record<StatisticType, StatisticTypeInfo> = {
  IntegerAmount: {
    type: 'IntegerAmount',
    label: 'Count',
    title: 'Count something',
    description: 'Push-ups, pages, cards',
    icon: ChartNoAxesColumn,
    origin: 'logged',
  },
  TimedAmounts: {
    type: 'TimedAmounts',
    label: 'Intake',
    title: 'Timed amounts',
    description: 'Amounts with a time, synced from an app (caffeine, nicotine)',
    icon: Coffee,
    origin: 'source',
  },
  Habit: {
    type: 'Habit',
    label: 'Habit',
    title: 'Track a habit',
    description: 'Done or not, once a day',
    icon: CircleCheck,
    origin: 'logged',
  },
  PointInTime: {
    type: 'PointInTime',
    label: 'Log a time',
    title: 'Log a time',
    description: 'When it happened: bedtime, wake-up',
    icon: Clock,
    origin: 'logged',
  },
  TimeSpans: {
    type: 'TimeSpans',
    label: 'Time spans',
    title: 'Measure time spans',
    description: 'Start/end windows like focus blocks',
    icon: SquareSplitHorizontal,
    origin: 'logged',
  },
  Summing: {
    type: 'Summing',
    label: 'Combined',
    title: 'Combine statistics',
    description: 'Adds up the daily values of others',
    icon: Sigma,
    origin: 'derived',
  },
  TimeDifference: {
    type: 'TimeDifference',
    label: 'Time difference',
    title: 'Time difference',
    description: 'Time between two logged times',
    icon: Hourglass,
    origin: 'derived',
  },
};

export const LOGGED_TYPES = Object.values(STATISTIC_TYPES).filter((info) => info.origin === 'logged');
export const DERIVED_TYPES = Object.values(STATISTIC_TYPES).filter((info) => info.origin === 'derived');

/** Goal editing isn't implemented for time differences; they are shown read-only. */
export function canEditGoals(type: StatisticType): boolean {
  return type !== 'TimeDifference';
}
