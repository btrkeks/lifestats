import {
  CalendarAccess,
  OccurrenceIdentity,
  occurrenceOf,
  OccurrenceProblem,
  PhoneCalendar,
  Times,
} from '@/models/calendar-sync';
import { NativeModule, requireNativeModule } from 'expo';
import { PermissionsAndroid } from 'react-native';
import { match, P } from 'ts-pattern';

type NativeOccurrence = {
  calendarId: number;
  accountName: string;
  eventId: number;
  instanceTime: number | null;
  syncId: string | null;
  rrule: string | null;
};
type NativeSpan = { beginMs: number; endMs: number };

// The native side returns plain maps; everything it says is parsed below before LiftLog trusts it.
declare class LiftLogCalendarModule extends NativeModule {
  listCalendars(): Promise<unknown>;
  findInstances(calendarId: number, accountName: string, beginMs: number, endMs: number): Promise<unknown>;
  inspectOccurrence(occurrence: NativeOccurrence): Promise<unknown>;
  moveOccurrence(occurrence: NativeOccurrence, next: NativeSpan, expected: NativeSpan): Promise<unknown>;
}

const PERMISSIONS = [PermissionsAndroid.PERMISSIONS.READ_CALENDAR, PermissionsAndroid.PERMISSIONS.WRITE_CALENDAR];

const id = P.when((value): value is number => Number.isSafeInteger(value));
const optionalText = P.union(P.string, P.nullish);

const problemOf = {
  noPermission: 'no-permission',
  calendarMissing: 'calendar-missing',
  readOnly: 'calendar-missing',
  gone: 'gone',
  duplicate: 'duplicate',
  unsupported: 'unsupported',
  unsynced: 'unsynced',
} as const satisfies Record<string, OccurrenceProblem>;
const problemStatus = P.union(
  'noPermission',
  'calendarMissing',
  'readOnly',
  'gone',
  'duplicate',
  'unsupported',
  'unsynced',
);

const calendarPattern = { id, displayName: optionalText, accountName: P.string };
const instancePattern = {
  eventId: id,
  instanceTime: P.union(id, P.nullish),
  syncId: optionalText,
  rrule: optionalText,
  rdate: optionalText,
  title: optionalText,
  beginMs: id,
  endMs: id,
  allDay: P.boolean,
  editable: P.boolean,
};

function unexpected(what: string, raw: unknown): never {
  throw new Error(`Unexpected answer from LiftLogCalendar.${what}: ${JSON.stringify(raw)}`);
}

const toNative = (identity: OccurrenceIdentity): NativeOccurrence => ({
  calendarId: identity.calendarId,
  accountName: identity.accountName,
  eventId: identity.eventId,
  instanceTime: identity.instanceTime ?? null,
  syncId: identity.syncId ?? null,
  rrule: identity.rrule ?? null,
});

const toSpan = (times: Times): NativeSpan => ({ beginMs: times.startMs, endMs: times.endMs });

const nonEmpty = (text: string | null | undefined) => (text ? text : undefined);

export function createPhoneCalendar(): PhoneCalendar {
  const native = requireNativeModule<LiftLogCalendarModule>('LiftLogCalendar');

  async function access(): Promise<CalendarAccess> {
    const granted = await Promise.all(PERMISSIONS.map((permission) => PermissionsAndroid.check(permission)));
    return granted.every(Boolean) ? 'granted' : 'denied';
  }

  return {
    supported: true,
    access,

    async requestAccess() {
      const answers = await PermissionsAndroid.requestMultiple(PERMISSIONS);
      const results = PERMISSIONS.map((permission) => answers[permission]);
      return results.every((result) => result === PermissionsAndroid.RESULTS.GRANTED)
        ? 'granted'
        : results.includes(PermissionsAndroid.RESULTS.NEVER_ASK_AGAIN)
          ? 'blocked'
          : 'denied';
    },

    async calendars() {
      const raw = await native.listCalendars();
      return match(raw)
        .with({ status: 'noPermission' }, () => [])
        .with({ status: 'ok', calendars: P.array(calendarPattern) }, ({ calendars }) =>
          calendars.map((c) => ({ id: c.id, displayName: c.displayName ?? '', accountName: c.accountName })),
        )
        .otherwise(() => unexpected('listCalendars', raw));
    },

    async occurrences({ calendarId, accountName }, window) {
      const raw = await native.findInstances(calendarId, accountName, window.startMs, window.endMs);
      return match(raw)
        .with({ status: P.union('noPermission', 'calendarMissing', 'readOnly') }, ({ status }) => ({
          kind: status === 'noPermission' ? ('no-permission' as const) : ('calendar-missing' as const),
        }))
        .with({ status: 'ok', instances: P.array(instancePattern) }, ({ instances }) => ({
          kind: 'found' as const,
          occurrences: instances.map((row) =>
            occurrenceOf({
              identity: {
                calendarId,
                accountName,
                eventId: row.eventId,
                instanceTime: row.instanceTime ?? undefined,
                syncId: nonEmpty(row.syncId),
                rrule: nonEmpty(row.rrule),
              },
              title: row.title ?? '',
              times: { startMs: row.beginMs, endMs: row.endMs },
              allDay: row.allDay,
              editable: row.editable,
              rdateOnly: !row.rrule && !!row.rdate,
            }),
          ),
        }))
        .otherwise(() => unexpected('findInstances', raw));
    },

    async inspect(identity) {
      const raw = await native.inspectOccurrence(toNative(identity));
      return match(raw)
        .with({ status: problemStatus }, ({ status }) => ({ kind: problemOf[status] }))
        .with({ status: 'ok', beginMs: id, endMs: id }, ({ beginMs, endMs }) => ({
          kind: 'found' as const,
          times: { startMs: beginMs, endMs },
        }))
        .otherwise(() => unexpected('inspectOccurrence', raw));
    },

    async move(identity, next, expected) {
      const raw = await native.moveOccurrence(toNative(identity), toSpan(next), toSpan(expected));
      return match(raw)
        .with({ status: problemStatus }, ({ status }) => ({ kind: problemOf[status] }))
        .with({ status: P.union('moved', 'unchanged') }, ({ status }) => ({ kind: status }))
        .with({ status: 'changed', beginMs: id, endMs: id }, ({ beginMs, endMs }) => ({
          kind: 'changed' as const,
          times: { startMs: beginMs, endMs },
        }))
        .otherwise(() => unexpected('moveOccurrence', raw));
    },
  };
}
