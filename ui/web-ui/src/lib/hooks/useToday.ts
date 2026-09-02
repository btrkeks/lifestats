'use client';

import { useState } from 'react';
import { todayIso } from '@/lib/dates';

/** Today's date (local time) as YYYY-MM-DD, stable for the component's lifetime. */
export function useToday(): string {
  const [today] = useState(() => todayIso());
  return today;
}
