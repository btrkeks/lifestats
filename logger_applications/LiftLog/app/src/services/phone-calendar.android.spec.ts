import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPhoneCalendar } from './phone-calendar.android';

const bridge = vi.hoisted(() => ({
  module: vi.fn(),
  calendars: vi.fn(),
  instances: vi.fn(),
  inspect: vi.fn(),
  move: vi.fn(),
}));
vi.mock('expo', () => ({ requireOptionalNativeModule: bridge.module }));
vi.mock('react-native', () => ({
  PermissionsAndroid: {
    PERMISSIONS: { READ_CALENDAR: 'read', WRITE_CALENDAR: 'write' },
    check: vi.fn(() => Promise.resolve(true)),
  },
}));

const row = {
  eventId: 17,
  instanceTime: 100,
  syncId: 'synced',
  rrule: 'FREQ=DAILY',
  rdate: null,
  title: 'Gym',
  organizer: 'me@example.com',
  sourceStartMs: 10,
  sourceDuration: 'P3600S',
  sourceTimezone: 'Europe/Berlin',
  exclusions: false,
  beginMs: 100,
  endMs: 200,
  allDay: false,
  editable: true,
};

describe('Android calendar bridge', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bridge.module.mockReturnValue({
      listCalendars: bridge.calendars,
      findInstances: bridge.instances,
      inspectOccurrence: bridge.inspect,
      moveOccurrence: bridge.move,
    });
  });

  it('keeps an old binary without the optional module usable', async () => {
    bridge.module.mockReturnValue(null);
    const calendar = createPhoneCalendar();
    expect(calendar.supported).toBe(false);
    expect(await calendar.access()).toBe('denied');
    expect(await calendar.calendars()).toEqual([]);
    expect(bridge.calendars).not.toHaveBeenCalled();
  });

  it('preserves the source identity across parsed native calls', async () => {
    bridge.instances.mockResolvedValue({ status: 'ok', instances: [row] });
    const calendar = createPhoneCalendar();
    const found = await calendar.occurrences(
      { calendarId: 7, accountName: 'me@example.com' },
      { startMs: 0, endMs: 1000 },
    );
    if (found.kind !== 'found' || !found.occurrences[0]) throw new Error('Expected one occurrence');
    const identity = found.occurrences[0].identity;
    bridge.move.mockResolvedValue({ status: 'moved' });
    await calendar.move(identity, { startMs: 300, endMs: 400 }, { startMs: 100, endMs: 200 });
    expect(bridge.move.mock.calls[0]?.[0]).toEqual({
      calendarId: 7,
      accountName: 'me@example.com',
      eventId: 17,
      instanceTime: 100,
      syncId: 'synced',
      rrule: 'FREQ=DAILY',
      title: 'Gym',
      organizer: 'me@example.com',
      sourceStartMs: 10,
      sourceDuration: 'P3600S',
      sourceTimezone: 'Europe/Berlin',
    });
  });

  it('counts excluded and all-day events as unsupported candidates', async () => {
    bridge.instances.mockResolvedValue({
      status: 'ok',
      instances: [
        { ...row, exclusions: true },
        { ...row, allDay: true, eventId: 18 },
      ],
    });
    const found = await createPhoneCalendar().occurrences(
      { calendarId: 7, accountName: 'me@example.com' },
      { startMs: 0, endMs: 1000 },
    );
    expect(found.kind === 'found' && found.occurrences.map((o) => o.problem)).toEqual(['unsupported', 'unsupported']);
  });

  it('rejects malformed native identity before it can be written', async () => {
    bridge.instances.mockResolvedValue({ status: 'ok', instances: [{ ...row, sourceStartMs: 'bad' }] });
    await expect(
      createPhoneCalendar().occurrences({ calendarId: 7, accountName: 'me@example.com' }, { startMs: 0, endMs: 1000 }),
    ).rejects.toThrow('Unexpected answer');
  });
});
