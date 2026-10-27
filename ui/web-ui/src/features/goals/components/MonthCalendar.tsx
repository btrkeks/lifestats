'use client';

import { useCallback, useEffect, useRef, type KeyboardEvent, type ReactNode } from 'react';
import { Popover as PopoverPrimitive } from '@base-ui/react/popover';
import { parseISO } from 'date-fns';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { cn } from 'cn';
import { Button } from '@/components/ui/button';
import { shiftIsoDate } from '@/lib/dates';
import type { Measure } from '@/lib/measure';
import { formatMonth, monthGrid, shiftMonth } from '@/features/calendar/grid';
import type { Day } from '../day';
import { DayCell, type DayCellProps } from './DayCell';

const WEEKDAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

/**
 * - `act`: click a day for its popover, Shift-click or drag for a range
 * - `pick`: click a day to choose it (the default goal's start)
 * - `inert`: read only
 */
export type CalendarMode = 'act' | 'pick' | 'inert';

const ARROW_STEP: Record<string, number> = {
  ArrowLeft: -1,
  ArrowRight: 1,
  ArrowUp: -7,
  ArrowDown: 7,
};

export function MonthCalendar({
  month,
  onMonthChange,
  today,
  days,
  measure,
  quiet,
  mode,
  hint,
  selection,
  target,
  preview,
  onPick,
  onDayClick,
  onDrag,
  popover,
  onPopoverClose,
  footer,
}: {
  month: string;
  onMonthChange: (month: string) => void;
  today: string;
  /** Days of the visible grid; undefined while loading */
  days: Day[] | undefined;
  measure: Measure;
  quiet: boolean;
  mode: CalendarMode;
  hint: ReactNode;
  selection: { from: string; to: string } | null;
  /** Day whose popover is open */
  target: string | null;
  preview: (day: Day) => DayCellProps['preview'];
  onPick: (date: string) => void;
  onDayClick: (date: string, extend: boolean) => void;
  onDrag: (anchor: string, end: string) => void;
  popover: ReactNode;
  onPopoverClose: () => void;
  footer: ReactNode;
}) {
  const grid = monthGrid(month);
  const byDate = new Map((days ?? []).map((day) => [day.date, day]));
  const gridRef = useRef<HTMLDivElement>(null);
  const drag = useRef<{ anchor: string; moved: boolean } | null>(null);
  const swallowClick = useRef(false);

  const cellFor = useCallback(
    (date: string | null) =>
      date
        ? (gridRef.current?.querySelector<HTMLButtonElement>(`[data-date="${date}"]`) ?? null)
        : null,
    []
  );

  useEffect(() => {
    const end = () => {
      if (drag.current?.moved) {
        // The click that follows this pointerup (if it lands on a day) ends the drag, not a click
        swallowClick.current = true;
        setTimeout(() => (swallowClick.current = false), 0);
      }
      drag.current = null;
    };
    window.addEventListener('pointerup', end);
    return () => window.removeEventListener('pointerup', end);
  }, []);

  const interactiveFor = (date: string) =>
    mode === 'pick' || (mode === 'act' && date <= today && byDate.has(date));

  const onKeyDown = (date: string) => (event: KeyboardEvent<HTMLButtonElement>) => {
    const step = ARROW_STEP[event.key];
    if (step === undefined) return;
    event.preventDefault();
    const next = cellFor(shiftIsoDate(date, step));
    if (next && !next.disabled) next.focus();
  };

  return (
    <div className="flex min-w-0 flex-col">
      <div className="mb-2 flex h-[34px] items-center justify-between gap-4">
        <div className="flex items-center gap-0.5">
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Previous month"
            onClick={() => onMonthChange(shiftMonth(month, -1))}
          >
            <ChevronLeft />
          </Button>
          <h2 className="min-w-[150px] text-center text-[15px] font-[620]" aria-live="polite">
            {formatMonth(month)}
          </h2>
          <Button
            variant="ghost"
            size="icon-sm"
            aria-label="Next month"
            onClick={() => onMonthChange(shiftMonth(month, 1))}
          >
            <ChevronRight />
          </Button>
          <Button
            variant="outline"
            size="xs"
            className="ml-2"
            onClick={() => onMonthChange(today.slice(0, 7))}
            disabled={month === today.slice(0, 7)}
          >
            Today
          </Button>
        </div>
        <div className="truncate text-[11.5px] text-faint">{hint}</div>
      </div>
      <div
        className="grid grid-cols-7 gap-1.5 px-0.5 pb-1.5 text-[11px] font-[550] tracking-[0.04em] text-faint uppercase"
        aria-hidden
      >
        {WEEKDAYS.map((weekday) => (
          <span key={weekday}>{weekday}</span>
        ))}
      </div>
      <div
        ref={gridRef}
        role="group"
        aria-label={`${formatMonth(month)} calendar`}
        className={cn(
          'grid grid-cols-7 gap-1.5',
          selection ? 'auto-rows-[112px]' : 'auto-rows-[118px]'
        )}
      >
        {grid.dates.map((date) => {
          const day = byDate.get(date) ?? null;
          const interactive = interactiveFor(date);
          return (
            <DayCell
              key={date}
              date={date}
              day={day}
              measure={measure}
              today={today}
              month={month}
              quiet={quiet}
              interactive={interactive}
              selected={selection !== null && date >= selection.from && date <= selection.to}
              targeted={target === date}
              preview={day ? preview(day) : null}
              onKeyDown={onKeyDown(date)}
              onPointerDown={(event) => {
                if (mode !== 'act' || event.button !== 0 || event.shiftKey || !interactive) return;
                drag.current = { anchor: date, moved: false };
              }}
              onPointerEnter={() => {
                const current = drag.current;
                if (!current || date > today || (date === current.anchor && !current.moved)) return;
                current.moved = true;
                onDrag(current.anchor, date);
              }}
              onClick={(event) => {
                if (swallowClick.current) {
                  swallowClick.current = false;
                  return;
                }
                if (mode === 'pick') onPick(date);
                else if (mode === 'act') onDayClick(date, event.shiftKey);
              }}
            />
          );
        })}
      </div>
      {footer}
      <PopoverPrimitive.Root
        key={target ?? 'none'}
        open={target !== null && popover !== null}
        onOpenChange={(open, details) => {
          if (open) return;
          // Clicking another day re-targets the popover instead of closing it
          const clicked = details.event?.target;
          if (
            details.reason === 'outside-press' &&
            clicked instanceof Element &&
            gridRef.current?.contains(clicked)
          ) {
            return;
          }
          onPopoverClose();
        }}
      >
        <PopoverPrimitive.Portal>
          <PopoverPrimitive.Positioner
            anchor={() => cellFor(target)}
            side={target && weekdayIndex(target) < 3 ? 'right' : 'left'}
            align="center"
            sideOffset={12}
            className="isolate z-50"
          >
            <PopoverPrimitive.Popup
              aria-label="Day actions"
              initialFocus={() =>
                document.querySelector<HTMLElement>('[data-day-action]:not(:disabled)') ?? true
              }
              finalFocus={() => cellFor(target)}
              className="w-[282px] origin-(--transform-origin) rounded-xl border border-line bg-card text-[13px] shadow-[0_18px_40px_-12px_rgba(28,27,25,0.28),0_2px_6px_rgba(28,27,25,0.06)] outline-none data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95"
            >
              {popover}
              <PopoverPrimitive.Arrow className="data-[side=left]:-right-[7px] data-[side=right]:-left-[7px] data-[side=top]:-bottom-[7px] data-[side=bottom]:-top-[7px]">
                <span className="block size-3 rotate-45 border-t border-r border-line bg-card in-data-[side=right]:-rotate-135 in-data-[side=top]:rotate-135 in-data-[side=bottom]:-rotate-45" />
              </PopoverPrimitive.Arrow>
            </PopoverPrimitive.Popup>
          </PopoverPrimitive.Positioner>
        </PopoverPrimitive.Portal>
      </PopoverPrimitive.Root>
    </div>
  );
}

/** 0 for Monday … 6 for Sunday */
function weekdayIndex(isoDate: string): number {
  return (parseISO(isoDate).getDay() + 6) % 7;
}
