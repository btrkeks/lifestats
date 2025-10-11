import { createElement } from 'react';
import type { ComponentProps, ReactElement } from 'react';
import type { StatisticType } from '@/types';

type IconProps = ComponentProps<'svg'>;

const baseIconProps: IconProps = {
  viewBox: '0 0 24 24',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.5,
  'aria-hidden': 'true',
};

const CountIcon = (props: IconProps): ReactElement =>
  createElement(
    'svg',
    { ...baseIconProps, ...props },
    createElement('path', {
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      d: 'M5 20v-6m7 6V8m7 12v-4',
    }),
    createElement('path', {
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      d: 'M4 20h16',
    })
  );

const HabitIcon = (props: IconProps): ReactElement =>
  createElement(
    'svg',
    { ...baseIconProps, ...props },
    createElement('path', {
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      d: 'M9 12l2 2 4-4m5 2a9 9 0 11-18 0 9 9 0 0118 0z',
    })
  );

const PointInTimeIcon = (props: IconProps): ReactElement =>
  createElement(
    'svg',
    { ...baseIconProps, ...props },
    createElement('circle', { cx: 12, cy: 12, r: 9 }),
    createElement('path', {
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      d: 'M12 7v5l3 3',
    })
  );

const TimeSpansIcon = (props: IconProps): ReactElement =>
  createElement(
    'svg',
    { ...baseIconProps, ...props },
    createElement('path', {
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      d: 'M5 6h4l2 4 2-4h6M5 18h4l2-4 2 4h6',
    })
  );

const SummingIcon = (props: IconProps): ReactElement =>
  createElement(
    'svg',
    { ...baseIconProps, ...props },
    createElement('path', {
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      d: 'M12 5v14M5 12h14',
    })
  );

const TimeDifferenceIcon = (props: IconProps): ReactElement =>
  createElement(
    'svg',
    { ...baseIconProps, ...props },
    createElement('path', {
      strokeLinecap: 'round',
      strokeLinejoin: 'round',
      d: 'M7 4h10l-4 6 4 6H7l4-6-4-6z',
    })
  );

export interface StatisticTypeDefinition {
  type: StatisticType;
  slug: string;
  title: string;
  description: string;
  icon: (props: IconProps) => ReactElement;
  href: string;
}

const buildDefinition = (
  type: StatisticType,
  slug: string,
  title: string,
  description: string,
  icon: (props: IconProps) => ReactElement
): StatisticTypeDefinition => ({
  type,
  slug,
  title,
  description,
  icon,
  href: `/statistics/new/${slug}`,
});

export const STATISTIC_TYPE_DEFINITIONS: StatisticTypeDefinition[] = [
  buildDefinition(
    'IntegerAmount',
    'integer-amount',
    'Count Something',
    'Track numbers like push-ups, pages read, or calories.',
    CountIcon
  ),
  buildDefinition(
    'Habit',
    'habit',
    'Track a Habit',
    'Record if you completed a habit on a given day.',
    HabitIcon
  ),
  buildDefinition(
    'PointInTime',
    'point-in-time',
    'Log a Time',
    'Capture the exact time something happened (like sleep or wake).',
    PointInTimeIcon
  ),
  buildDefinition(
    'TimeSpans',
    'time-spans',
    'Measure Time Spans',
    'Track start/end windows such as focused work blocks.',
    TimeSpansIcon
  ),
  buildDefinition(
    'Summing',
    'summing',
    'Combine Statistics',
    'Create a derived metric that sums other statistics.',
    SummingIcon
  ),
  buildDefinition(
    'TimeDifference',
    'time-difference',
    'Calculate Time Difference',
    'Measure the difference between two point-in-time statistics.',
    TimeDifferenceIcon
  ),
];
