import { DayOfWeek } from '@js-joda/core';
import { ActionCreatorWithPreparedPayload, createAction, UnknownAction } from '@reduxjs/toolkit';
import { CalendarChoice } from '@/models/calendar-sync';
import {
  boolCodec,
  calendarChoiceCodec,
  Codec,
  colorSchemeSeedCodec,
  ColorSchemeSeed,
  dayOfWeekCodec,
  PlansSortOrder,
  plansSortOrderCodec,
  stringCodec,
  ThemeMode,
  themeModeCodec,
} from './codecs';

/**
 * One descriptor per preference is the single source of truth: the state field,
 * its default, hydration, and persistence are all derived from this.
 */
export interface PrefDescriptor<T> {
  default: T;
  /** When present, the key participates in generic hydrate + generic persist. */
  codec?: Codec<T>;
  /** Defaults to the registry key, keeping the on-disk name identical. */
  storageKey?: string;
  /**
   * `false` => a bespoke effect owns write-back (permission gates, dev guards,
   * composite keys), so the generic persist effect skips it.
   */
  persist?: boolean;
  /**
   * `'manual'` => the init effect reads it explicitly (sync reads, composite
   * keys, values composed from several keys).
   */
  hydrate?: 'generic' | 'manual';
  /** Read via `getItemSync` during hydration. */
  sync?: boolean;
}

const pref = <T>(descriptor: PrefDescriptor<T>): PrefDescriptor<T> => descriptor;

export const preferenceRegistry = {
  useImperialUnits: pref({ default: false, codec: boolCodec }),
  showBodyweight: pref({ default: true, codec: boolCodec }),
  restNotifications: pref({ default: true, codec: boolCodec }),
  restTimersEnabled: pref({ default: true, codec: boolCodec }),
  notesExpandedByDefault: pref({ default: true, codec: boolCodec }),
  keepScreenAwakeDuringWorkout: pref({ default: true, codec: boolCodec }),
  showPostWorkoutSummary: pref({ default: false, codec: boolCodec }),
  trueBlackDarkTheme: pref({ default: false, codec: boolCodec }),
  colorSchemeSeed: pref<ColorSchemeSeed>({ default: 'default', codec: colorSchemeSeedCodec }),
  themeMode: pref<ThemeMode>({ default: 'system', codec: themeModeCodec }),
  plansSortOrder: pref<PlansSortOrder>({ default: 'name', codec: plansSortOrderCodec }),
  firstDayOfWeek: pref<DayOfWeek>({ default: DayOfWeek.SUNDAY, codec: dayOfWeekCodec }),
  lifestatsSyncEnabled: pref({ default: false, codec: boolCodec }),
  lifestatsSyncUrl: pref<string | undefined>({ default: 'https://laptop-1.tail87da75.ts.net', codec: stringCodec }),
  calendarSyncEnabled: pref({ default: false, codec: boolCodec }),
  calendarSyncCalendar: pref<CalendarChoice | undefined>({ default: undefined, codec: calendarChoiceCodec }),
  calendarSyncEventTitle: pref<string | undefined>({ default: 'Gym', codec: stringCodec }),

  // Sync read + legacy rewrite; write-back also drives Tolgee/RTL.
  preferredLanguage: pref<string | undefined>({
    default: undefined,
    codec: stringCodec,
    persist: false,
    hydrate: 'manual',
    sync: true,
  }),
} satisfies Record<string, PrefDescriptor<unknown>>;

export type PreferenceRegistry = typeof preferenceRegistry;
export type PrefKey = keyof PreferenceRegistry;
export type PrefValue<K extends PrefKey> = PreferenceRegistry[K] extends PrefDescriptor<infer T> ? T : never;

export const preferenceKeys = Object.keys(preferenceRegistry) as PrefKey[];

const capitalize = <S extends string>(value: S): Capitalize<S> =>
  (value.charAt(0).toUpperCase() + value.slice(1)) as Capitalize<S>;

// Marks a preference-setter action so a single matcher reducer/effect can handle
// every key generically while the wire `type` stays `settings/set<Key>`.
export interface PreferenceActionMeta {
  prefKey: PrefKey;
}

// The generated setters preserve the historical action names and payload types
// (`setColorSchemeSeed(value)` etc.) so no call site changes.
export type PreferenceSetters = {
  [K in PrefKey as `set${Capitalize<K & string>}`]: ActionCreatorWithPreparedPayload<
    [PrefValue<K>],
    PrefValue<K>,
    `settings/set${Capitalize<K & string>}`,
    never,
    PreferenceActionMeta
  >;
};

export const preferenceSetters = Object.fromEntries(
  preferenceKeys.map((key) => [
    `set${capitalize(key)}`,
    createAction(`settings/set${capitalize(key)}`, (value: unknown) => ({
      payload: value,
      meta: { prefKey: key } satisfies PreferenceActionMeta,
    })),
  ]),
) as unknown as PreferenceSetters;

export function setterForKey<K extends PrefKey>(key: K): PreferenceSetters[`set${Capitalize<K & string>}`] {
  return preferenceSetters[`set${capitalize(key)}`];
}

// Builds a setter action for a key whose type is only known generically. A
// generically-indexed action creator can't be called directly (TS infers a
// `never` parameter), so the setter is cast to a plain function of its value.
export function buildPreferenceAction<K extends PrefKey>(key: K, value: PrefValue<K>): PreferenceAction {
  const create = setterForKey(key) as unknown as (value: PrefValue<K>) => PreferenceAction;
  return create(value);
}

export interface PreferenceAction extends UnknownAction {
  payload: unknown;
  meta: PreferenceActionMeta;
}

export function isPreferenceAction(action: UnknownAction): action is PreferenceAction {
  const meta = (action as { meta?: { prefKey?: unknown } }).meta;
  return !!meta && typeof meta.prefKey === 'string' && meta.prefKey in preferenceRegistry;
}
