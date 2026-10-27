'use client';

import { ArrowDown, ArrowUp } from 'lucide-react';
import { cn } from 'cn';
import type { Measure } from '@/lib/measure';
import { STATISTIC_TYPES } from '@/lib/statisticTypes';
import type { StatisticMetadata } from '@/types';
import type { TrendPoint, TrendReadout } from '../insights';
import { TrendChart } from './TrendChart';

export interface Figure {
  label: string;
  value: string;
}

function Kpi({ figure }: { figure: Figure }) {
  return (
    <div className="flex flex-col items-end">
      <span className="text-[11px] font-medium text-faint">{figure.label}</span>
      <span className="text-[15px] font-semibold whitespace-nowrap tabular">{figure.value}</span>
    </div>
  );
}

function Readout({ readout }: { readout: TrendReadout }) {
  const Arrow = readout.delta?.direction === 'down' ? ArrowDown : ArrowUp;
  return (
    <div className="flex min-w-[112px] flex-col items-start">
      <span className="text-[11px] font-medium text-faint">{readout.label}</span>
      <span className="text-[15px] font-semibold tabular">{readout.value}</span>
      {readout.delta && (
        <span
          className={cn(
            'mt-px inline-flex items-center gap-[3px] text-[11px] font-[550] whitespace-nowrap',
            readout.delta.better ? 'text-met' : 'text-missed'
          )}
        >
          <Arrow className="size-[9px]" strokeWidth={2.5} aria-hidden />
          {readout.delta.text}
        </span>
      )}
    </div>
  );
}

/** Title, the 12-week trend and the key figures above the calendar. */
export function DetailHeader({
  statistic,
  measure,
  hasGoal,
  trend,
  figures,
}: {
  statistic: StatisticMetadata;
  measure: Measure;
  hasGoal: boolean;
  trend: { points: TrendPoint[]; level: number | null; readout: TrendReadout } | null;
  figures: Figure[];
}) {
  return (
    <div className="flex items-end border-b border-line pb-4">
      <div className="min-w-0 shrink pr-4">
        <p className="mb-1 truncate text-xs text-muted-foreground">
          {statistic.category} · {STATISTIC_TYPES[statistic.statistic_type].label} · #{statistic.id}
        </p>
        <h1 className="flex items-center gap-3 text-2xl font-[650] tracking-[-0.02em]">
          <span className="truncate">{statistic.name}</span>
          {!hasGoal && (
            <span className="shrink-0 rounded-full border border-dashed border-[#CFCBC2] px-2 py-0.5 text-xs font-[550] tracking-normal text-muted-foreground">
              No goal
            </span>
          )}
        </h1>
      </div>
      <div className="mr-6 ml-auto flex shrink-0 items-end gap-3 border-r border-line pr-6">
        {trend ? (
          <>
            <TrendChart
              points={trend.points}
              measure={measure}
              level={trend.level}
              label={`${statistic.name} per day, last 12 weeks`}
            />
            <Readout readout={trend.readout} />
          </>
        ) : (
          <div className="flex h-[52px] w-[300px] flex-col justify-end">
            <p className="mb-2 text-center text-xs text-faint">Trend appears once it has a goal</p>
            <div className="border-t border-dashed border-[#D6D2CA]" />
          </div>
        )}
      </div>
      <div className="flex gap-[26px]">
        {figures.map((figure) => (
          <Kpi key={figure.label} figure={figure} />
        ))}
      </div>
    </div>
  );
}
