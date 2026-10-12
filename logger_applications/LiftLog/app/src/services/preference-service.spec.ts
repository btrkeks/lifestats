import { afterEach, describe, expect, it, vi } from 'vitest';
import { PreferenceService } from '@/services/preference-service';
import { KeyValueStore } from '@/services/key-value-store';
import { PrefKey } from '@/store/settings/registry';
import { DayOfWeek } from '@js-joda/core';

// Characterization tests pinning the exact value read for a missing key and for
// representative stored strings, and the exact string written back - asserted
// through the generic getPreference/setPreference and the bespoke named methods.
// This is the guarantee that the registry refactor never changes what a user's
// stored data means or loses it.

function makeKvStore(initial: Record<string, string> = {}) {
  const map = new Map<string, string>(Object.entries(initial));
  const store = {
    map,
    getItem: vi.fn((key: string) => Promise.resolve(map.has(key) ? map.get(key) : undefined)),
    getItemSync: vi.fn((key: string) => (map.has(key) ? map.get(key) : undefined)),
    getItemBytes: vi.fn(() => Promise.resolve(undefined)),
    setItem: vi.fn((key: string, value: string) => {
      map.set(key, value);
      return Promise.resolve();
    }),
    removeItem: vi.fn((key: string) => {
      map.delete(key);
      return Promise.resolve();
    }),
  };
  return store;
}

function makeService(initial: Record<string, string> = {}) {
  const store = makeKvStore(initial);
  const service = new PreferenceService(store as unknown as KeyValueStore);
  return { service, store };
}

afterEach(() => {
  vi.unstubAllGlobals();
});

// ─── Boolean preferences ────────────────────────────────────────────────────

interface BoolPref {
  key: PrefKey;
  default: boolean;
}

const booleanPrefs: BoolPref[] = [
  { key: 'useImperialUnits', default: false },
  { key: 'showBodyweight', default: true },
  { key: 'restNotifications', default: true },
  { key: 'restTimersEnabled', default: true },
  { key: 'notesExpandedByDefault', default: true },
  { key: 'keepScreenAwakeDuringWorkout', default: true },
  { key: 'showPostWorkoutSummary', default: false },
  { key: 'trueBlackDarkTheme', default: false },
];

describe('PreferenceService - boolean preferences', () => {
  for (const pref of booleanPrefs) {
    describe(pref.key, () => {
      it(`defaults to ${pref.default} when unset`, async () => {
        const { service } = makeService();
        expect(await service.getPreference(pref.key)).toBe(pref.default);
      });

      it("reads 'True' as true and 'False' as false", async () => {
        expect(await makeService({ [pref.key]: 'True' }).service.getPreference(pref.key)).toBe(true);
        expect(await makeService({ [pref.key]: 'False' }).service.getPreference(pref.key)).toBe(false);
      });

      it("reads any non-'True' string as false", async () => {
        expect(await makeService({ [pref.key]: 'true' }).service.getPreference(pref.key)).toBe(false);
        expect(await makeService({ [pref.key]: 'garbage' }).service.getPreference(pref.key)).toBe(false);
      });

      it("writes 'True' / 'False' to the key", async () => {
        const { service, store } = makeService();
        await service.setPreference(pref.key, true);
        expect(store.setItem).toHaveBeenCalledWith(pref.key, 'True');
        await service.setPreference(pref.key, false);
        expect(store.setItem).toHaveBeenCalledWith(pref.key, 'False');
      });
    });
  }
});

// ─── colorSchemeSeed ────────────────────────────────────────────────────────

describe('PreferenceService - colorSchemeSeed', () => {
  it("defaults to 'default' when unset", async () => {
    expect(await makeService().service.getPreference('colorSchemeSeed')).toBe('default');
  });

  it('reads a valid six-digit hex (either case) verbatim', async () => {
    expect(await makeService({ colorSchemeSeed: '#aabbcc' }).service.getPreference('colorSchemeSeed')).toBe('#aabbcc');
    expect(await makeService({ colorSchemeSeed: '#AABBCC' }).service.getPreference('colorSchemeSeed')).toBe('#AABBCC');
  });

  it("falls back to 'default' for anything not a six-digit hex", async () => {
    expect(await makeService({ colorSchemeSeed: 'default' }).service.getPreference('colorSchemeSeed')).toBe('default');
    expect(await makeService({ colorSchemeSeed: 'red' }).service.getPreference('colorSchemeSeed')).toBe('default');
    expect(await makeService({ colorSchemeSeed: '#xyz' }).service.getPreference('colorSchemeSeed')).toBe('default');
    expect(await makeService({ colorSchemeSeed: '#abc' }).service.getPreference('colorSchemeSeed')).toBe('default');
  });

  it('writes the value verbatim', async () => {
    const { service, store } = makeService();
    await service.setPreference('colorSchemeSeed', '#123456');
    expect(store.setItem).toHaveBeenCalledWith('colorSchemeSeed', '#123456');
    await service.setPreference('colorSchemeSeed', 'default');
    expect(store.setItem).toHaveBeenCalledWith('colorSchemeSeed', 'default');
  });
});

// ─── themeMode ──────────────────────────────────────────────────────────────

describe('PreferenceService - themeMode', () => {
  it("defaults to 'system' when unset or unrecognised", async () => {
    expect(await makeService().service.getPreference('themeMode')).toBe('system');
    expect(await makeService({ themeMode: 'Dark' }).service.getPreference('themeMode')).toBe('system');
    expect(await makeService({ themeMode: 'garbage' }).service.getPreference('themeMode')).toBe('system');
  });

  it('reads and writes each mode verbatim', async () => {
    for (const mode of ['system', 'light', 'dark'] as const) {
      expect(await makeService({ themeMode: mode }).service.getPreference('themeMode')).toBe(mode);
      const { service, store } = makeService();
      await service.setPreference('themeMode', mode);
      expect(store.setItem).toHaveBeenCalledWith('themeMode', mode);
    }
  });
});

// ─── firstDayOfWeek ───────────────────────────────────────────────────────────

describe('PreferenceService - firstDayOfWeek', () => {
  const days: [string, DayOfWeek][] = [
    ['sunday', DayOfWeek.SUNDAY],
    ['monday', DayOfWeek.MONDAY],
    ['tuesday', DayOfWeek.TUESDAY],
    ['wednesday', DayOfWeek.WEDNESDAY],
    ['thursday', DayOfWeek.THURSDAY],
    ['friday', DayOfWeek.FRIDAY],
    ['saturday', DayOfWeek.SATURDAY],
  ];

  it('defaults to SUNDAY when unset or unknown', async () => {
    expect(await makeService().service.getPreference('firstDayOfWeek')).toBe(DayOfWeek.SUNDAY);
    expect(await makeService({ firstDayOfWeek: 'someday' }).service.getPreference('firstDayOfWeek')).toBe(
      DayOfWeek.SUNDAY,
    );
  });

  for (const [name, day] of days) {
    it(`reads '${name}' (case-insensitively) as ${day.toString()}`, async () => {
      expect(await makeService({ firstDayOfWeek: name }).service.getPreference('firstDayOfWeek')).toBe(day);
      expect(await makeService({ firstDayOfWeek: name.toUpperCase() }).service.getPreference('firstDayOfWeek')).toBe(
        day,
      );
    });
  }

  it('writes the DayOfWeek name', async () => {
    const { service, store } = makeService();
    await service.setPreference('firstDayOfWeek', DayOfWeek.MONDAY);
    expect(store.setItem).toHaveBeenCalledWith('firstDayOfWeek', 'MONDAY');
  });
});

// ─── preferredLanguage (sync, legacy rewrite, remove-on-undefined) ────────────

describe('PreferenceService - preferredLanguage', () => {
  it('defaults to undefined and reads a stored code', () => {
    expect(makeService().service.getPreferredLanguage()).toBeUndefined();
    expect(makeService({ preferredLanguage: 'en' }).service.getPreferredLanguage()).toBe('en');
  });

  it("rewrites legacy 'zh_Hans' to 'zh-hans' on read", () => {
    const { service, store } = makeService({ preferredLanguage: 'zh_Hans' });
    expect(service.getPreferredLanguage()).toBe('zh-hans');
    expect(store.setItem).toHaveBeenCalledWith('preferredLanguage', 'zh-hans');
  });

  it('writes a code and removes the key when cleared', async () => {
    const { service, store } = makeService();
    await service.setPreferredLanguage('fr');
    expect(store.setItem).toHaveBeenCalledWith('preferredLanguage', 'fr');
    await service.setPreferredLanguage(undefined);
    expect(store.removeItem).toHaveBeenCalledWith('preferredLanguage');
  });
});
