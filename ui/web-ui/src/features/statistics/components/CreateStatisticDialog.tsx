'use client';

import { useId, useState } from 'react';
import { Command as CommandPrimitive } from 'cmdk';
import { Check, Search, X } from 'lucide-react';
import { cn } from 'cn';
import { Button } from '@/components/ui/button';
import { Command, CommandItem, CommandList } from '@/components/ui/command';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { errorMessage } from '@/lib/api';
import {
  DERIVED_TYPES,
  LOGGED_TYPES,
  STATISTIC_TYPES,
  type StatisticTypeInfo,
} from '@/lib/statisticTypes';
import type { CreateStatisticRequest, StatisticMetadata, StatisticType } from '@/types';
import { useCreateStatistic, useLoggableStatistics } from '../hooks/useStatistics';
import { searchStatistics } from '../search';

const SOURCE_HELP: Partial<Record<StatisticType, string>> = {
  Summing:
    "Each day's value is the sum of the selected statistics on that day. You can give it a goal after creating it.",
  TimeDifference:
    "Pick two logged times, the start first. Each day's value is the time between them. Goals for time differences can't be edited yet.",
};

function TypeOption({
  info,
  selected,
  onSelect,
}: {
  info: StatisticTypeInfo;
  selected: boolean;
  onSelect: () => void;
}) {
  const Icon = info.icon;
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      onClick={onSelect}
      className={cn(
        'flex items-center gap-[11px] rounded-[9px] px-2.5 py-2 text-left outline-none focus-visible:ring-3 focus-visible:ring-brand/30',
        selected
          ? 'bg-card shadow-[0_0_0_1.5px_var(--brand),0_4px_10px_-6px_rgba(59,76,214,0.4)]'
          : 'hover:bg-card/70'
      )}
    >
      <span
        className={cn(
          'grid size-8 shrink-0 place-items-center rounded-lg border',
          selected ? 'border-brand bg-brand text-white' : 'border-line bg-card text-ink-2'
        )}
      >
        <Icon className="size-4" aria-hidden />
      </span>
      <span className="flex flex-col">
        <b className="text-[13.5px] font-[580]">{info.title}</b>
        <small className="text-[11.5px] text-muted-foreground">{info.description}</small>
      </span>
    </button>
  );
}

function SourcePicker({
  statistics,
  type,
  selected,
  onChange,
}: {
  statistics: StatisticMetadata[];
  type: StatisticType;
  selected: number[];
  onChange: (ids: number[]) => void;
}) {
  const [query, setQuery] = useState('');
  const candidates =
    type === 'TimeDifference'
      ? statistics.filter((s) => s.statistic_type === 'PointInTime')
      : statistics;
  const matches = searchStatistics(candidates, query);
  const byId = new Map(statistics.map((statistic) => [statistic.id, statistic]));
  const limit = type === 'TimeDifference' ? 2 : Infinity;
  const toggle = (id: number) =>
    onChange(
      selected.includes(id)
        ? selected.filter((s) => s !== id)
        : selected.length < limit
          ? [...selected, id]
          : selected
    );

  return (
    <>
      <p className="mt-3 mb-1.5 text-[13px] font-[550] text-ink-2">
        Source statistics{' '}
        <span className="text-muted-foreground">· {selected.length} selected</span>
      </p>
      {selected.length > 0 && (
        <div className="mb-2 flex flex-wrap gap-1.5">
          {selected.map((id) => (
            <span
              key={id}
              className="inline-flex h-[26px] items-center gap-1 rounded-md bg-brand-tint pr-[3px] pl-2 text-xs font-[550] text-[#2d3aa8]"
            >
              {byId.get(id)?.name ?? `#${id}`}
              <button
                type="button"
                aria-label={`Remove ${byId.get(id)?.name ?? id}`}
                onClick={() => toggle(id)}
                className="grid size-5 place-items-center rounded text-[#5562c9] hover:bg-brand-line/40"
              >
                <X className="size-3" />
              </button>
            </span>
          ))}
        </div>
      )}
      <Command
        label="Source statistics"
        shouldFilter={false}
        loop
        className="rounded-[10px]! border border-[#DEDAD2] p-0"
      >
        <label className="flex h-[38px] items-center gap-2 border-b border-line-2 px-3 text-muted-foreground">
          <Search className="size-4 shrink-0" aria-hidden />
          <CommandPrimitive.Input
            value={query}
            onValueChange={setQuery}
            placeholder="Search statistics"
            aria-label="Search source statistics"
            className="min-w-0 flex-1 bg-transparent font-medium text-ink outline-none placeholder:font-normal placeholder:text-faint"
          />
          <span className="text-[11.5px] text-faint tabular">
            {matches.length} of {candidates.length}
          </span>
        </label>
        <CommandList className="h-[230px] max-h-none p-1">
          {matches.map((statistic) => {
            const on = selected.includes(statistic.id);
            return (
              <CommandItem
                key={statistic.id}
                value={String(statistic.id)}
                onSelect={() => toggle(statistic.id)}
                className="h-8 gap-2.5 rounded-[7px] px-[9px] data-selected:bg-line-2"
              >
                <span
                  className={cn(
                    'grid size-4 shrink-0 place-items-center rounded-[4px] border-[1.5px]',
                    on ? 'border-brand bg-brand text-white' : 'border-[#CFCBC2]'
                  )}
                  aria-hidden
                >
                  {on && <Check className="size-3" strokeWidth={3} />}
                </span>
                <span className="flex-1 truncate text-[13px]">
                  {statistic.name}
                  {on && <span className="sr-only"> (selected)</span>}
                </span>
                <span className="text-[11.5px] text-faint tabular">#{statistic.id}</span>
              </CommandItem>
            );
          })}
        </CommandList>
      </Command>
    </>
  );
}

function validate(
  type: StatisticType,
  name: string,
  category: string,
  sources: number[]
): string | null {
  if (!name.trim()) return 'Give the statistic a name.';
  if (!category.trim()) return 'Pick or type a category.';
  if (type === 'Summing' && sources.length === 0) return 'Select at least one source statistic.';
  if (type === 'TimeDifference' && sources.length !== 2) return 'Select exactly two logged times.';
  return null;
}

/** "New statistic": choose what it measures, then name it. */
export function CreateStatisticDialog({
  open,
  onOpenChange,
  categories,
  onCreated,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  categories: string[];
  onCreated: (statistic: StatisticMetadata) => void;
}) {
  const [type, setType] = useState<StatisticType>('IntegerAmount');
  const [name, setName] = useState('');
  const [category, setCategory] = useState('');
  const [sources, setSources] = useState<number[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const derived = STATISTIC_TYPES[type].derived;
  const loggable = useLoggableStatistics(open && derived);
  const create = useCreateStatistic();
  const nameId = useId();
  const categoryId = useId();
  const listId = useId();

  const reset = () => {
    setType('IntegerAmount');
    setName('');
    setCategory('');
    setSources([]);
    setProblem(null);
    create.reset();
  };

  const submit = (event: React.FormEvent) => {
    event.preventDefault();
    const issue = validate(type, name, category, sources);
    setProblem(issue);
    if (issue) return;
    const request: CreateStatisticRequest = {
      name: name.trim(),
      category: category.trim(),
      statistic_type: type,
      ...(derived ? { source_statistic_ids: sources } : {}),
    };
    create.mutate(request, {
      onSuccess: (statistic) => {
        onCreated(statistic);
        onOpenChange(false);
        reset();
      },
    });
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        onOpenChange(next);
        if (!next) reset();
      }}
    >
      <DialogContent
        showCloseButton={false}
        className="flex h-[min(628px,calc(100vh-2rem))] w-[900px] max-w-[calc(100%-2rem)] flex-col gap-0 overflow-hidden rounded-2xl p-0 shadow-[0_30px_80px_-20px_rgba(0,0,0,0.45)] sm:max-w-[900px]"
      >
        <form onSubmit={submit} className="flex min-h-0 flex-1 flex-col">
          <div className="flex items-start justify-between border-b border-line px-[22px] pt-5 pb-4">
            <div>
              <DialogTitle className="text-lg font-[650] tracking-[-0.01em]">
                New statistic
              </DialogTitle>
              <DialogDescription className="mt-[3px] text-[13px] text-muted-foreground">
                Choose what it measures, then name it.
              </DialogDescription>
            </div>
            <DialogClose
              render={<Button type="button" variant="ghost" size="icon-sm" aria-label="Close" />}
            >
              <X />
            </DialogClose>
          </div>
          <div className="flex min-h-0 flex-1">
            <div
              role="radiogroup"
              aria-label="What it measures"
              className="flex w-[290px] shrink-0 flex-col gap-0.5 border-r border-line bg-[#FAF9F6] p-2.5"
            >
              <p className="px-2.5 pt-2.5 pb-1.5 text-[11px] font-semibold tracking-[0.05em] text-faint uppercase">
                Logged
              </p>
              {LOGGED_TYPES.map((info) => (
                <TypeOption
                  key={info.type}
                  info={info}
                  selected={type === info.type}
                  onSelect={() => setType(info.type)}
                />
              ))}
              <p className="px-2.5 pt-4 pb-1.5 text-[11px] font-semibold tracking-[0.05em] text-faint uppercase">
                Derived from other statistics
              </p>
              {DERIVED_TYPES.map((info) => (
                <TypeOption
                  key={info.type}
                  info={info}
                  selected={type === info.type}
                  onSelect={() => {
                    setType(info.type);
                    setSources([]);
                  }}
                />
              ))}
            </div>
            <div className="flex min-w-0 flex-1 flex-col overflow-y-auto px-[22px] pt-3 pb-4">
              <div className="grid grid-cols-[1fr_190px] gap-3">
                <div>
                  <label
                    htmlFor={nameId}
                    className="mt-3 mb-1.5 block text-[13px] font-[550] text-ink-2"
                  >
                    Name
                  </label>
                  <Input
                    id={nameId}
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="Morning push-ups"
                  />
                </div>
                <div>
                  <label
                    htmlFor={categoryId}
                    className="mt-3 mb-1.5 block text-[13px] font-[550] text-ink-2"
                  >
                    Category
                  </label>
                  <Input
                    id={categoryId}
                    list={listId}
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                    placeholder="Pick or type one"
                  />
                  <datalist id={listId}>
                    {categories.map((c) => (
                      <option key={c} value={c} />
                    ))}
                  </datalist>
                </div>
              </div>
              {derived ? (
                loggable.data ? (
                  <SourcePicker
                    statistics={loggable.data}
                    type={type}
                    selected={sources}
                    onChange={setSources}
                  />
                ) : (
                  <p className="mt-4 text-[13px] text-muted-foreground">Loading statistics…</p>
                )
              ) : (
                <p className="mt-3 text-[13px] text-muted-foreground">
                  You can give it a goal after creating it.
                </p>
              )}
              {SOURCE_HELP[type] && (
                <p className="mt-2.5 text-[13px] text-muted-foreground">{SOURCE_HELP[type]}</p>
              )}
            </div>
          </div>
          <div className="flex items-center justify-end gap-2 border-t border-line bg-[#FAF9F6] px-[22px] py-3.5">
            {(problem || create.error) && (
              <p className="mr-auto text-xs text-missed" role="alert">
                {problem ?? errorMessage(create.error)}
              </p>
            )}
            <DialogClose render={<Button type="button" variant="ghost" />}>Cancel</DialogClose>
            <Button type="submit" disabled={create.isPending}>
              {create.isPending ? 'Creating…' : 'Create statistic'}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
