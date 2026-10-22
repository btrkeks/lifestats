import { DayOfWeek } from '@js-joda/core';
import { match, P } from 'ts-pattern';
import { CalendarChoice } from '@/models/calendar-sync';

export type ColorSchemeSeed = 'default' | `#${string}`;

export type ThemeMode = 'system' | 'light' | 'dark';

export type PlansSortOrder = 'name' | 'recent';

// A codec maps between a preference's runtime value and its on-disk string.
// `deserialize` returns undefined when the key is absent or unparseable, so the
// caller falls back to the descriptor default. `serialize` returns undefined to
// signal the key should be removed rather than written.
export interface Codec<T> {
  deserialize(raw: string | undefined): T | undefined;
  serialize(value: T): string | undefined;
}

// Booleans persist as 'True'/'False' - the casing is historical, don't change it.
export const boolCodec: Codec<boolean> = {
  deserialize: (raw) => (raw === undefined ? undefined : raw === 'True'),
  serialize: (value) => (value ? 'True' : 'False'),
};

export const stringCodec: Codec<string | undefined> = {
  deserialize: (raw) => raw,
  serialize: (value) => value,
};

export const colorSchemeSeedCodec: Codec<ColorSchemeSeed> = {
  deserialize: (raw) =>
    match(raw)
      .returnType<ColorSchemeSeed>()
      .with(P.string.regex(/^#[0-9a-fA-F]{6}$/), (v) => v as ColorSchemeSeed)
      .otherwise(() => 'default'),
  serialize: (value) => value,
};

export const themeModeCodec: Codec<ThemeMode> = {
  deserialize: (raw) =>
    match(raw)
      .with('light', 'dark', 'system', (v) => v)
      .otherwise(() => undefined),
  serialize: (value) => value,
};

export const plansSortOrderCodec: Codec<PlansSortOrder> = {
  deserialize: (raw) =>
    match(raw)
      .with('name', 'recent', (v) => v)
      .otherwise(() => undefined),
  serialize: (value) => value,
};

export const dayOfWeekCodec: Codec<DayOfWeek> = {
  deserialize: (raw) =>
    match(raw?.toLowerCase())
      .with('sunday', () => DayOfWeek.SUNDAY)
      .with('monday', () => DayOfWeek.MONDAY)
      .with('tuesday', () => DayOfWeek.TUESDAY)
      .with('wednesday', () => DayOfWeek.WEDNESDAY)
      .with('thursday', () => DayOfWeek.THURSDAY)
      .with('friday', () => DayOfWeek.FRIDAY)
      .with('saturday', () => DayOfWeek.SATURDAY)
      .otherwise(() => undefined),
  serialize: (value) => value.name(),
};

// The calendar that calendar sync moves events in, as JSON. Its account travels with its id, so a calendar
// id that another account reuses later is never taken for the chosen calendar.
export const calendarChoiceCodec: Codec<CalendarChoice | undefined> = {
  deserialize: (raw) => {
    const json = parseJson(raw);
    return match(json)
      .returnType<CalendarChoice | undefined>()
      .with(
        { id: P.number.int(), displayName: P.string, accountName: P.string },
        ({ id, displayName, accountName }) => ({ id, displayName, accountName }),
      )
      .otherwise(() => undefined);
  },
  serialize: (value) => (value ? JSON.stringify(value) : undefined),
};

function parseJson(raw: string | undefined): unknown {
  try {
    return raw === undefined ? undefined : JSON.parse(raw);
  } catch {
    return undefined;
  }
}
