import {
  ChartNoAxesColumn,
  CircleCheck,
  Clock,
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
  derived: boolean;
}

export const STATISTIC_TYPES: Record<StatisticType, StatisticTypeInfo> = {
  IntegerAmount: {
    type: 'IntegerAmount',
    label: 'Count',
    title: 'Count something',
    description: 'Push-ups, pages, cards',
    icon: ChartNoAxesColumn,
    derived: false,
  },
  Habit: {
    type: 'Habit',
    label: 'Habit',
    title: 'Track a habit',
    description: 'Done or not, once a day',
    icon: CircleCheck,
    derived: false,
  },
  PointInTime: {
    type: 'PointInTime',
    label: 'Log a time',
    title: 'Log a time',
    description: 'When it happened: bedtime, wake-up',
    icon: Clock,
    derived: false,
  },
  TimeSpans: {
    type: 'TimeSpans',
    label: 'Time spans',
    title: 'Measure time spans',
    description: 'Start/end windows like focus blocks',
    icon: SquareSplitHorizontal,
    derived: false,
  },
  Summing: {
    type: 'Summing',
    label: 'Combined',
    title: 'Combine statistics',
    description: 'Adds up the daily values of others',
    icon: Sigma,
    derived: true,
  },
  TimeDifference: {
    type: 'TimeDifference',
    label: 'Time difference',
    title: 'Time difference',
    description: 'Time between two logged times',
    icon: Hourglass,
    derived: true,
  },
};

export const LOGGED_TYPES = Object.values(STATISTIC_TYPES).filter((info) => !info.derived);
export const DERIVED_TYPES = Object.values(STATISTIC_TYPES).filter((info) => info.derived);

/** Goal editing isn't implemented for time differences; they are shown read-only. */
export function canEditGoals(type: StatisticType): boolean {
  return type !== 'TimeDifference';
}
