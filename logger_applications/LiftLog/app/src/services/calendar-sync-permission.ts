import { CalendarAccess, CalendarChoice, PhoneCalendar } from '@/models/calendar-sync';

export async function allowCalendarAccess({
  access,
  phoneCalendar,
  openSettings,
  setAccess,
  setCalendars,
  retry,
}: {
  access: CalendarAccess | undefined;
  phoneCalendar: Pick<PhoneCalendar, 'requestAccess' | 'calendars'>;
  openSettings: () => Promise<void>;
  setAccess: (access: CalendarAccess) => void;
  setCalendars: (calendars: CalendarChoice[]) => void;
  retry: () => void;
}): Promise<void> {
  if (access === 'blocked') {
    await openSettings();
    return;
  }
  const granted = await phoneCalendar.requestAccess();
  setAccess(granted);
  if (granted === 'blocked') {
    await openSettings();
    return;
  }
  if (granted === 'granted') {
    setCalendars(await phoneCalendar.calendars());
    retry();
  }
}
