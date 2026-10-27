'use client';

import { useState } from 'react';
import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import type { StatisticMetadata } from '@/types';
import { formatGoal } from '@/lib/goalFormat';
import { measureOf } from '@/lib/measure';
import { parseStatisticDefaultGoal } from '@/lib/schemas';
import { hasDefaultGoal } from '@/lib/statisticGroups';
import { searchStatistics } from '../search';

function Item({
  statistic,
  onOpen,
}: {
  statistic: StatisticMetadata;
  onOpen: (id: number) => void;
}) {
  const goal = parseStatisticDefaultGoal(statistic);
  return (
    <CommandItem
      value={String(statistic.id)}
      onSelect={() => onOpen(statistic.id)}
      className="gap-3 py-2"
    >
      <span className="min-w-0 flex-1">
        <span className="block truncate font-[560]">{statistic.name}</span>
        <span className="block truncate text-[11.5px] text-muted-foreground">
          {statistic.category} · #{statistic.id}
        </span>
      </span>
      <span className="shrink-0 text-xs text-muted-foreground tabular">
        {goal ? formatGoal(goal, measureOf(statistic)) : 'No goal'}
      </span>
    </CommandItem>
  );
}

/** The header's "Find a statistic…" palette (⌘/Ctrl K). */
export function StatisticSearchDialog({
  open,
  onOpenChange,
  statistics,
  onOpen,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  statistics: StatisticMetadata[];
  onOpen: (id: number) => void;
}) {
  const [query, setQuery] = useState('');
  const matches = searchStatistics(statistics, query);
  const open_ = (id: number) => {
    onOpen(id);
    onOpenChange(false);
    setQuery('');
  };

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Find a statistic"
      description="Search statistics by name or category"
      className="sm:max-w-xl"
    >
      <Command label="Find a statistic" shouldFilter={false} loop>
        <CommandInput value={query} onValueChange={setQuery} placeholder="Find a statistic…" />
        <CommandList className="max-h-[420px]">
          <CommandEmpty>No statistic matches “{query}”.</CommandEmpty>
          <CommandGroup heading="Active goals">
            {matches.filter(hasDefaultGoal).map((statistic) => (
              <Item key={statistic.id} statistic={statistic} onOpen={open_} />
            ))}
          </CommandGroup>
          <CommandGroup heading="Without a goal">
            {matches
              .filter((statistic) => !hasDefaultGoal(statistic))
              .map((statistic) => (
                <Item key={statistic.id} statistic={statistic} onOpen={open_} />
              ))}
          </CommandGroup>
        </CommandList>
      </Command>
    </CommandDialog>
  );
}
