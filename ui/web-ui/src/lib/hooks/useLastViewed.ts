'use client';

import { useCallback, useEffect, useState } from 'react';

const KEY = 'lifestats.lastViewedStatistic';

/**
 * The statistic the user last opened, remembered in this browser.
 * `undefined` until read (after hydration), `null` when nothing is stored.
 */
export function useLastViewed(): [number | null | undefined, (id: number) => void] {
  const [id, setId] = useState<number | null | undefined>(undefined);

  useEffect(() => {
    const stored = Number(window.localStorage.getItem(KEY));
    setId(
      Number.isInteger(stored) && stored >= 0 && window.localStorage.getItem(KEY) !== null
        ? stored
        : null
    );
  }, []);

  const remember = useCallback((next: number) => {
    window.localStorage.setItem(KEY, String(next));
    setId(next);
  }, []);

  return [id, remember];
}
