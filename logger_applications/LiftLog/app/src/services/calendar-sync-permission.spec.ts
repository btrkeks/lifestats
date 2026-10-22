import { describe, expect, it, vi } from 'vitest';
import { CalendarAccess, CalendarChoice } from '@/models/calendar-sync';
import { allowCalendarAccess } from '@/services/calendar-sync-permission';

const calendars: CalendarChoice[] = [{ id: 3, displayName: 'Training', accountName: 'me@example.com' }];

function button(access: CalendarAccess | undefined, requested: CalendarAccess) {
  const phoneCalendar = {
    requestAccess: vi.fn(() => Promise.resolve(requested)),
    calendars: vi.fn(() => Promise.resolve(calendars)),
  };
  return {
    access,
    phoneCalendar,
    openSettings: vi.fn(() => Promise.resolve()),
    setAccess: vi.fn<(access: CalendarAccess) => void>(),
    setCalendars: vi.fn<(calendars: CalendarChoice[]) => void>(),
    retry: vi.fn(),
  };
}

describe('Allow calendar access', () => {
  it('opens settings once on the same invocation when denied access is permanently blocked', async () => {
    const command = button('denied', 'blocked');

    await allowCalendarAccess(command);

    expect(command.phoneCalendar.requestAccess).toHaveBeenCalledOnce();
    expect(command.setAccess).toHaveBeenCalledWith('blocked');
    expect(command.openSettings).toHaveBeenCalledOnce();
    expect(command.phoneCalendar.calendars).not.toHaveBeenCalled();
    expect(command.retry).not.toHaveBeenCalled();
  });

  it('leaves an ordinary denial on the page without opening settings', async () => {
    const command = button('denied', 'denied');
    await allowCalendarAccess(command);
    expect(command.setAccess).toHaveBeenCalledWith('denied');
    expect(command.openSettings).not.toHaveBeenCalled();
    expect(command.phoneCalendar.calendars).not.toHaveBeenCalled();
    expect(command.retry).not.toHaveBeenCalled();
  });

  it('refreshes the picker and retries pending workouts after a grant', async () => {
    const command = button('denied', 'granted');
    await allowCalendarAccess(command);
    expect(command.setAccess).toHaveBeenCalledWith('granted');
    expect(command.phoneCalendar.calendars).toHaveBeenCalledOnce();
    expect(command.setCalendars).toHaveBeenCalledWith(calendars);
    expect(command.retry).toHaveBeenCalledOnce();
    expect(command.openSettings).not.toHaveBeenCalled();
  });

  it('opens settings directly when the page already knows permission is blocked', async () => {
    const command = button('blocked', 'blocked');
    await allowCalendarAccess(command);
    expect(command.openSettings).toHaveBeenCalledOnce();
    expect(command.phoneCalendar.requestAccess).not.toHaveBeenCalled();
    expect(command.retry).not.toHaveBeenCalled();
  });
});
