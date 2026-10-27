'use client';

import { Command as CommandPrimitive } from 'cmdk';
import { Search, X } from 'lucide-react';
import { cn } from 'cn';
import { Command, CommandGroup, CommandItem, CommandList } from '@/components/ui/command';
import { Kbd } from '@/components/ui/kbd';
import { Segmented } from '@/components/Segmented';
import type { StatisticMetadata } from '@/types';
import { formatGoal } from '@/lib/goalFormat';
import { measureOf } from '@/lib/measure';
import { parseStatisticDefaultGoal } from '@/lib/schemas';
import { hasDefaultGoal } from '@/lib/statisticGroups';
import { STATISTIC_TYPES } from '@/lib/statisticTypes';
import { highlightParts, searchStatistics } from '../search';

export type GoalFilter = 'all' | 'none' | 'has';

function Highlighted({ text, query }: { text: string; query: string }) {
  return (
    <>
      {highlightParts(text, query).map((part, i) =>
        part.match ? (
          <mark key={i} className="rounded-[2px] bg-[#FFE9A8] px-px text-inherit">
            {part.text}
          </mark>
        ) : (
          <span key={i}>{part.text}</span>
        )
      )}
    </>
  );
}

function ResultRow({
  statistic,
  query,
  selected,
  onOpen,
}: {
  statistic: StatisticMetadata;
  query: string;
  selected: boolean;
  onOpen: (id: number) => void;
}) {
  const type = STATISTIC_TYPES[statistic.statistic_type];
  const Icon = type.icon;
  const goal = parseStatisticDefaultGoal(statistic);
  return (
    <CommandItem
      value={String(statistic.id)}
      onSelect={() => onOpen(statistic.id)}
      aria-current={selected ? 'true' : undefined}
      className={cn(
        'relative mx-0 gap-[11px] rounded-[10px] p-2.5 text-[13px]',
        selected ? 'bg-brand-tint data-selected:bg-brand-tint' : 'data-selected:bg-line-2'
      )}
    >
      {selected && (
        <span
          className="absolute top-2.5 bottom-2.5 -left-2 w-[3px] rounded-r-[3px] bg-brand"
          aria-hidden
        />
      )}
      <span className="grid size-[30px] shrink-0 place-items-center rounded-lg border border-line bg-card text-ink-2">
        <Icon className="size-4" aria-hidden />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-[560]">
          <Highlighted text={statistic.name} query={query} />
        </span>
        <span className="mt-0.5 block truncate text-[11.5px] text-muted-foreground">
          <Highlighted text={statistic.category} query={query} /> · {type.label} · #{statistic.id}
        </span>
      </span>
      {goal ? (
        <span className="shrink-0 text-[12.5px] font-[550] text-ink-2 tabular">
          {formatGoal(goal, measureOf(statistic))}
        </span>
      ) : (
        <span className="shrink-0 rounded-full border border-dashed border-[#CFCBC2] px-2 py-0.5 text-[11px] font-[550] text-muted-foreground">
          No goal
        </span>
      )}
    </CommandItem>
  );
}

const FILTER_NOUN: Record<GoalFilter, string> = {
  all: 'statistics',
  none: 'without a goal',
  has: 'with a goal',
};

export function AllStatisticsPane({
  statistics,
  query,
  onQueryChange,
  filter,
  onFilterChange,
  selectedId,
  onOpen,
  autoFocus,
}: {
  statistics: StatisticMetadata[];
  query: string;
  onQueryChange: (query: string) => void;
  filter: GoalFilter;
  onFilterChange: (filter: GoalFilter) => void;
  selectedId: number | null;
  onOpen: (id: number) => void;
  autoFocus: boolean;
}) {
  const withGoal = statistics.filter(hasDefaultGoal);
  const withoutGoal = statistics.filter((statistic) => !hasDefaultGoal(statistic));
  const inFilter = filter === 'all' ? statistics : filter === 'none' ? withoutGoal : withGoal;
  const others = filter === 'none' ? withGoal : filter === 'has' ? withoutGoal : [];
  const matches = searchStatistics(inFilter, query);
  const otherMatches = query.trim() ? searchStatistics(others, query) : [];

  const heading = query.trim()
    ? `${matches.length} ${matches.length === 1 ? 'match' : 'matches'} among ${inFilter.length} ${FILTER_NOUN[filter]}`
    : filter === 'all'
      ? `${statistics.length} statistics`
      : `${inFilter.length} statistics ${FILTER_NOUN[filter]}`;

  return (
    <Command
      label="Search statistics"
      shouldFilter={false}
      loop
      className="flex min-h-0 flex-1 flex-col rounded-none! bg-transparent p-0"
    >
      <div className="px-3.5 pt-3.5 pb-2.5">
        <label className="flex h-10 items-center gap-2 rounded-lg border border-[#DEDAD2] bg-card px-2.5 text-muted-foreground focus-within:border-brand focus-within:ring-3 focus-within:ring-brand-tint">
          <Search className="size-4 shrink-0" aria-hidden />
          <CommandPrimitive.Input
            value={query}
            onValueChange={onQueryChange}
            autoFocus={autoFocus}
            placeholder="Search statistics"
            aria-label="Search statistics"
            className="min-w-0 flex-1 bg-transparent text-[15px] font-medium text-ink outline-none placeholder:font-normal placeholder:text-faint"
          />
          {query && (
            <button
              type="button"
              aria-label="Clear search"
              onClick={() => onQueryChange('')}
              className="grid size-6 place-items-center rounded-md hover:bg-line-2"
            >
              <X className="size-4" />
            </button>
          )}
        </label>
      </div>
      <div className="px-3.5 pb-3">
        <Segmented
          label="Filter by goal"
          value={filter}
          onChange={onFilterChange}
          options={[
            { value: 'all', label: 'All' },
            {
              value: 'none',
              label: (
                <>
                  No goal{' '}
                  <span className="ml-0.5 text-[11px] text-faint">{withoutGoal.length}</span>
                </>
              ),
            },
            {
              value: 'has',
              label: (
                <>
                  Has goal <span className="ml-0.5 text-[11px] text-faint">{withGoal.length}</span>
                </>
              ),
            },
          ]}
        />
      </div>
      <CommandList className="max-h-none min-h-0 flex-1 px-2 pb-2">
        <CommandGroup
          heading={heading}
          className="p-0 **:[[cmdk-group-heading]]:px-2.5 **:[[cmdk-group-heading]]:pt-0 **:[[cmdk-group-heading]]:pb-1.5 **:[[cmdk-group-heading]]:text-[11.5px] **:[[cmdk-group-heading]]:text-faint"
        >
          {matches.map((statistic) => (
            <ResultRow
              key={statistic.id}
              statistic={statistic}
              query={query}
              selected={statistic.id === selectedId}
              onOpen={onOpen}
            />
          ))}
        </CommandGroup>
        {otherMatches.length > 0 && (
          <CommandGroup
            heading={`Also matching, ${FILTER_NOUN[filter === 'none' ? 'has' : 'none']}`}
            className="mt-3 p-0 **:[[cmdk-group-heading]]:px-2.5 **:[[cmdk-group-heading]]:pb-1.5 **:[[cmdk-group-heading]]:text-[11.5px] **:[[cmdk-group-heading]]:text-faint"
          >
            {otherMatches.map((statistic) => (
              <ResultRow
                key={statistic.id}
                statistic={statistic}
                query={query}
                selected={statistic.id === selectedId}
                onOpen={onOpen}
              />
            ))}
          </CommandGroup>
        )}
      </CommandList>
      <p className="mx-[18px] mt-auto mb-4 text-[11.5px] leading-[1.8] text-faint">
        Search matches names and categories. <Kbd>↑</Kbd> <Kbd>↓</Kbd> to move, <Kbd>Enter</Kbd> to
        open.
      </p>
    </Command>
  );
}
