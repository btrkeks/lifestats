'use client';

import { useEffect, useMemo, useState } from 'react';
import { cn } from 'cn';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { AppHeader } from '@/components/layout/AppHeader';
import { errorMessage } from '@/lib/api';
import { useLastViewed } from '@/lib/hooks/useLastViewed';
import { useToday } from '@/lib/hooks/useToday';
import { hasDefaultGoal, sortStatistics } from '@/lib/statisticGroups';
import { ActiveGoalsList, type ActiveGoal } from '@/features/goals/components/ActiveGoalsList';
import { GoalDetail } from '@/features/goals/components/GoalDetail';
import { useStreaks, useTodays } from '@/features/goals/hooks/useGoals';
import {
  AllStatisticsPane,
  type GoalFilter,
} from '@/features/statistics/components/AllStatisticsPane';
import { CreateStatisticDialog } from '@/features/statistics/components/CreateStatisticDialog';
import { StatisticSearchDialog } from '@/features/statistics/components/StatisticSearchDialog';
import { useStatistics } from '@/features/statistics/hooks/useStatistics';

type Tab = 'active' | 'all';

function TabCount({ count }: { count: number }) {
  return (
    <span className="rounded-[10px] bg-line-2 px-1.5 text-[11px] leading-[17px] text-muted-foreground group-data-active/tab:bg-ink group-data-active/tab:text-white">
      {count}
    </span>
  );
}

const TAB_CLASS = cn(
  'group/tab h-auto flex-none gap-1.5 rounded-none border-0 border-b-2 border-transparent px-2.5 pt-[7px] pb-2.5 text-[13px] font-[550] text-muted-foreground',
  '-mb-px after:hidden data-active:border-ink data-active:text-ink hover:text-ink'
);

/** The whole app: goal list or all statistics on the left, the selected statistic on the right. */
export function Workspace() {
  const today = useToday();
  const statistics = useStatistics();
  const all = useMemo(() => sortStatistics(statistics.data ?? []), [statistics.data]);
  const active = useMemo(() => all.filter(hasDefaultGoal), [all]);
  const todays = useTodays(active, today);
  const streaks = useStreaks(active.map((statistic) => statistic.id));
  const [lastViewed, remember] = useLastViewed();
  const [tab, setTab] = useState<Tab>('active');
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<GoalFilter>('all');
  const [searchOpen, setSearchOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);

  const goals: ActiveGoal[] = active.map((statistic, i) => ({
    statistic,
    today: todays[i]?.data?.[0],
    streak: streaks[i]?.data,
  }));

  // Last viewed, else the first goal still open (or missed) today
  const todaysSettled = todays.every((result) => !result.isPending);
  const fallbackId =
    goals.find((goal) => goal.today?.status === 'open' || goal.today?.status === 'missed')
      ?.statistic.id ??
    active[0]?.id ??
    all[0]?.id ??
    null;
  const selectedId =
    lastViewed !== undefined &&
    lastViewed !== null &&
    all.some((statistic) => statistic.id === lastViewed)
      ? lastViewed
      : lastViewed !== undefined && todaysSettled
        ? fallbackId
        : null;
  const selected = all.find((statistic) => statistic.id === selectedId) ?? null;

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        setSearchOpen(true);
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);

  /** Opens a statistic from outside the left pane (search, a newly created one). */
  const reveal = (id: number) => {
    remember(id);
    const statistic = all.find((s) => s.id === id);
    if (statistic && hasDefaultGoal(statistic)) {
      setTab('active');
    } else {
      setTab('all');
      setFilter('all');
      setQuery('');
    }
  };

  const categories = useMemo(() => [...new Set(all.map((statistic) => statistic.category))], [all]);

  return (
    <div className="flex h-dvh flex-col">
      <AppHeader
        today={today}
        onSearch={() => setSearchOpen(true)}
        onCreate={() => setCreateOpen(true)}
      />
      <div className="flex min-h-0 flex-1">
        <aside
          className="flex w-[360px] shrink-0 flex-col border-r border-line bg-card"
          aria-label="Statistics"
        >
          <Tabs
            value={tab}
            onValueChange={(value) => setTab(value === 'all' ? 'all' : 'active')}
            className="flex min-h-0 flex-1 flex-col gap-0"
          >
            <TabsList
              variant="line"
              className="h-auto w-full justify-start gap-1 rounded-none border-b border-line px-3.5 pt-3 pb-0"
            >
              <TabsTrigger value="active" className={TAB_CLASS}>
                Active goals <TabCount count={active.length} />
              </TabsTrigger>
              <TabsTrigger value="all" className={TAB_CLASS}>
                All statistics <TabCount count={all.length} />
              </TabsTrigger>
            </TabsList>
            <TabsContent value="active" className="flex min-h-0 flex-col data-hidden:hidden">
              {statistics.isError ? (
                <p className="p-4 text-[13px] text-missed">
                  Couldn&apos;t load statistics: {errorMessage(statistics.error)}
                </p>
              ) : (
                <ActiveGoalsList
                  goals={goals}
                  selectedId={selectedId}
                  onSelect={remember}
                  withoutGoalCount={all.length - active.length}
                  onShowWithoutGoal={() => {
                    setFilter('none');
                    setTab('all');
                  }}
                />
              )}
            </TabsContent>
            <TabsContent value="all" className="flex min-h-0 flex-col data-hidden:hidden">
              <AllStatisticsPane
                statistics={all}
                query={query}
                onQueryChange={setQuery}
                filter={filter}
                onFilterChange={setFilter}
                selectedId={selectedId}
                onOpen={remember}
                autoFocus
              />
            </TabsContent>
          </Tabs>
        </aside>
        <main className="min-w-0 flex-1 overflow-y-auto">
          {selected ? (
            <GoalDetail key={selected.id} statistic={selected} today={today} />
          ) : (
            <p className="p-8 text-[13px] text-muted-foreground">
              {statistics.isPending || !todaysSettled
                ? 'Loading…'
                : 'No statistics yet. Create one to get started.'}
            </p>
          )}
        </main>
      </div>
      <StatisticSearchDialog
        open={searchOpen}
        onOpenChange={setSearchOpen}
        statistics={all}
        onOpen={reveal}
      />
      <CreateStatisticDialog
        open={createOpen}
        onOpenChange={setCreateOpen}
        categories={categories}
        onCreated={(statistic) => reveal(statistic.id)}
      />
    </div>
  );
}
