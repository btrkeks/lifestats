import { PhoneCalendar } from '@/models/calendar-sync';

export function unsupportedPhoneCalendar(): PhoneCalendar {
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
