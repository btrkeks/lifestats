'use client';

import { Bar, Cell, ComposedChart, Line, ReferenceLine, Tooltip, XAxis, YAxis } from 'recharts';
import { ChartContainer, type ChartConfig } from '@/components/ui/chart';
import { formatDay, formatShortMonth } from '@/lib/dates';
import { formatCount, formatSeconds, type Measure } from '@/lib/measure';
import type { DayStatus } from '../day';
import { formatMinutes, type TrendPoint } from '../insights';

const STATUS_FILL: Record<DayStatus, string> = {
  met: 'var(--met)',
  missed: 'var(--missed)',
  open: 'var(--open)',
  suppressed: '#D6D2CA',
  none: '#D6D2CA',
  future: '#D6D2CA',
};

const config = { y: { label: 'Value' } } satisfies ChartConfig;

function formatLevel(measure: Measure, value: number): string {
  switch (measure.kind) {
    case 'count':
      return formatCount(value);
    case 'duration':
      return formatSeconds(value);
    case 'clock':
      return formatMinutes(value);
    case 'habit':
    case 'spans':
      return `${value}%`;
  }
}

/** First date of each month in the points, for the axis ticks. */
function monthTicks(points: TrendPoint[]): string[] {
  return points
    .filter((point, i) => i === 0 || point.date.slice(0, 7) !== points[i - 1].date.slice(0, 7))
    .map((p) => p.date);
}

/**
 * The header's 12-week trend: bars for amounts and percentages, dots for
 * times of day, coloured by status, with a dashed line at the goal.
 */
export function TrendChart({
  points,
  measure,
  level,
  label,
}: {
  points: TrendPoint[];
  measure: Measure;
  level: number | null;
  label: string;
}) {
  const values = points.flatMap((point) => (point.y === null ? [] : [point.y]));
  const asDots = measure.kind === 'clock';
  const extent = [...values, ...(level === null ? [] : [level])];
  const domain: [number, number] = asDots
    ? [Math.min(...extent) - 10, Math.max(...extent) + 10]
    : [0, Math.max(1, ...extent) * 1.08];

  return (
    <ChartContainer
      config={config}
      className="aspect-auto h-[52px] w-[242px]"
      role="img"
      aria-label={label}
    >
      <ComposedChart
        data={points}
        margin={{ top: 4, right: 34, bottom: 0, left: 0 }}
        barCategoryGap={0.6}
      >
        <XAxis
          dataKey="date"
          ticks={monthTicks(points)}
          tickFormatter={formatShortMonth}
          interval={0}
          tick={{ fontSize: 9.5, fill: 'var(--faint)', textAnchor: 'start' }}
          tickLine={{ stroke: '#D6D2CA' }}
          tickSize={3}
          axisLine={{ stroke: '#D6D2CA' }}
          height={14}
        />
        <YAxis hide domain={domain} />
        <Tooltip
          cursor={false}
          isAnimationActive={false}
          content={({ active, payload }) => {
            const point = payload?.[0]?.payload;
            if (!active || !point || point.y === null) return null;
            return (
              <div className="rounded-md border border-line bg-card px-2 py-1 text-[11px] shadow-sm">
                <span className="text-muted-foreground">{formatDay(point.date)}</span>{' '}
                <b className="font-semibold tabular">{formatLevel(measure, point.y)}</b>
              </div>
            );
          }}
        />
        {asDots ? (
          <Line
            dataKey="y"
            stroke="#CFCBC2"
            strokeWidth={1}
            isAnimationActive={false}
            connectNulls={false}
            dot={({ cx, cy, index }) => {
              const point = points[index];
              return (
                <circle
                  key={index}
                  cx={cx}
                  cy={cy}
                  r={1.8}
                  fill={!point || point.y === null ? 'transparent' : STATUS_FILL[point.status]}
                />
              );
            }}
            activeDot={{ r: 3 }}
          />
        ) : (
          <Bar dataKey="y" isAnimationActive={false} minPointSize={1}>
            {points.map((point) => (
              <Cell key={point.date} fill={STATUS_FILL[point.status]} />
            ))}
          </Bar>
        )}
        {level !== null && (
          <ReferenceLine
            y={level}
            stroke="var(--ink-2)"
            strokeOpacity={0.55}
            strokeDasharray="2 2"
            ifOverflow="extendDomain"
            label={{
              value: formatLevel(measure, level),
              position: 'right',
              fontSize: 9.5,
              fill: 'var(--muted-foreground)',
              fontWeight: 550,
            }}
          />
        )}
      </ComposedChart>
    </ChartContainer>
  );
}
