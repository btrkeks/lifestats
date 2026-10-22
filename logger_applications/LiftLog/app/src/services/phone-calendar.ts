import { PhoneCalendar } from '@/models/calendar-sync';

/** iOS and web have no calendar sync; `phone-calendar.android.ts` is the real one. */
export function createPhoneCalendar(): PhoneCalendar {
  return {
    supported: false,
    access: () => Promise.resolve('denied'),
    requestAccess: () => Promise.resolve('denied'),
    calendars: () => Promise.resolve([]),
    occurrences: () => Promise.resolve({ kind: 'no-permission' }),
    inspect: () => Promise.resolve({ kind: 'no-permission' }),
    move: () => Promise.resolve({ kind: 'no-permission' }),
  };
}
