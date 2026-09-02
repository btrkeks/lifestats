'use client';

import { useEffect, useRef } from 'react';
import type { DailyAchievement } from '@/types';
import { createSuppressedRequest } from '@/lib/schemas';
import { formatDayLabel } from '@/lib/dates';
import { useDeleteDailyGoal, useSetDailyGoal } from '../hooks/useGoals';
import { DAY_STATUS_LABEL, type DayStatus } from '../dayStatus';

interface DayPopoverProps {
  statisticId: number;
  day: DailyAchievement;
  status: DayStatus;
  align: 'left' | 'right';
  onClose: () => void;
  onEditGoal: () => void;
}

const ACTION_CLASSES =
  'w-full rounded-md px-3 py-1.5 text-left text-sm font-medium transition disabled:cursor-not-allowed disabled:opacity-50';

/** Actions for one day of the strip: suppress, set a custom goal, or remove the override. */
export function DayPopover({ statisticId, day, status, align, onClose, onEditGoal }: DayPopoverProps) {
  const ref = useRef<HTMLDivElement>(null);
  const setGoal = useSetDailyGoal(statisticId);
  const deleteGoal = useDeleteDailyGoal(statisticId);
  const isBusy = setGoal.isPending || deleteGoal.isPending;
  const error = setGoal.error ?? deleteGoal.error;

  useEffect(() => {
    const onPointerDown = (event: PointerEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) onClose();
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [onClose]);

  const suppress = () =>
    setGoal.mutate({ date: day.date, request: createSuppressedRequest() }, { onSuccess: onClose });
  const removeOverride = () => deleteGoal.mutate(day.date, { onSuccess: onClose });

  return (
    <div
      ref={ref}
      role="dialog"
      aria-label={`Goal actions for ${formatDayLabel(day.date)}`}
      className={`absolute top-full z-20 mt-2 w-56 rounded-lg border border-gray-200 bg-white p-3 shadow-lg ${
        align === 'left' ? 'left-0' : 'right-0'
      }`}
    >
      <p className="text-sm font-semibold text-gray-900">{formatDayLabel(day.date)}</p>
      <p className="text-xs text-gray-500">
        {DAY_STATUS_LABEL[status]}
        {day.percentage_achieved !== null && ` · ${day.percentage_achieved}%`}
        {day.override === 'goal' && ' · custom goal'}
      </p>

      <div className="mt-3 flex flex-col gap-1">
        {status !== 'suppressed' && (
          <button
            type="button"
            onClick={suppress}
            disabled={isBusy}
            className={`${ACTION_CLASSES} bg-amber-50 text-amber-900 hover:bg-amber-100`}
          >
            Suppress goal
          </button>
        )}
        <button
          type="button"
          onClick={onEditGoal}
          disabled={isBusy}
          className={`${ACTION_CLASSES} text-gray-800 hover:bg-gray-100`}
        >
          Set custom goal…
        </button>
        {day.override !== 'none' && (
          <button
            type="button"
            onClick={removeOverride}
            disabled={isBusy}
            className={`${ACTION_CLASSES} text-gray-800 hover:bg-gray-100`}
          >
            Remove override
          </button>
        )}
      </div>

      {isBusy && <p className="mt-2 text-xs text-gray-500">Saving…</p>}
      {error && <p className="mt-2 text-xs text-rose-600">{error.message}</p>}
    </div>
  );
}
