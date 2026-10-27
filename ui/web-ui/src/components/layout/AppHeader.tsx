'use client';

import { useEffect, useState } from 'react';
import { CalendarDays, Plus, Search } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Kbd } from '@/components/ui/kbd';
import { formatDayWithYear } from '@/lib/dates';

function Logo() {
  return (
    <span className="flex h-4 items-end gap-0.5" aria-hidden>
      <i className="h-2 w-1 rounded-[1.5px] bg-ink" />
      <i className="h-4 w-1 rounded-[1.5px] bg-brand" />
      <i className="h-[11px] w-1 rounded-[1.5px] bg-ink" />
    </span>
  );
}

/** "⌘K" on Apple devices, "Ctrl K" elsewhere (decided after hydration). */
function useSearchShortcutLabel(): string {
  const [label, setLabel] = useState('Ctrl K');
  useEffect(() => {
    if (/mac|iphone|ipad/i.test(navigator.userAgent)) setLabel('⌘K');
  }, []);
  return label;
}

export function AppHeader({
  today,
  onSearch,
  onCreate,
}: {
  today: string;
  onSearch: () => void;
  onCreate: () => void;
}) {
  const shortcut = useSearchShortcutLabel();
  return (
    <header className="flex h-[52px] shrink-0 items-center gap-5 border-b border-line bg-card pr-5 pl-[22px]">
      <div className="flex w-[338px] items-center gap-[9px] text-[15px] font-[650] tracking-[-0.01em]">
        <Logo />
        LifeStats
      </div>
      <button
        type="button"
        onClick={onSearch}
        className="flex h-8 w-[340px] items-center gap-2 rounded-lg border border-line bg-background pr-2 pl-[11px] text-muted-foreground outline-none hover:border-[#d9d5cc] focus-visible:ring-3 focus-visible:ring-brand/30"
      >
        <Search className="size-[15px]" aria-hidden />
        <span className="flex-1 text-left">Find a statistic…</span>
        <Kbd>{shortcut}</Kbd>
      </button>
      <div className="ml-auto flex items-center gap-4">
        <span className="flex items-center gap-1.5 text-[12.5px] text-muted-foreground">
          <CalendarDays className="size-[15px]" aria-hidden />
          {/* The prerendered page carries the build day; the client renders today */}
          <span suppressHydrationWarning>{formatDayWithYear(today)}</span>
        </span>
        <Button size="sm" onClick={onCreate}>
          <Plus data-icon="inline-start" /> New statistic
        </Button>
      </div>
    </header>
  );
}
