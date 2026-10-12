import { describe, expect, it, vi } from 'vitest';

import { createAddEffectTestBed } from '@/utils/__test__/add-effect-testbed';
import { applySettingsEffects } from '@/store/settings/effects';
import { setColorSchemeSeed, setPreferredLanguage, settingsReducer } from '@/store/settings';

describe('settings slice - generated preference actions', () => {
  it('applies a generated setter through the matcher reducer', () => {
    const state = settingsReducer(undefined, setColorSchemeSeed('#abcdef'));
    expect(state.colorSchemeSeed).toBe('#abcdef');
  });

  it('keeps the historical action type string', () => {
    expect(setColorSchemeSeed('#abcdef').type).toBe('settings/setColorSchemeSeed');
  });

  it('seeds initial state from the registry defaults (drift fixed to true)', () => {
    const state = settingsReducer(undefined, { type: '@@init' });
    expect(state.notesExpandedByDefault).toBe(true);
    expect(state.keepScreenAwakeDuringWorkout).toBe(true);
    expect(state.isHydrated).toBe(false);
  });
});

function makeTestBed(isHydrated: boolean) {
  const preferenceService = {
    setPreference: vi.fn(() => Promise.resolve()),
    setPreferredLanguage: vi.fn(() => Promise.resolve()),
  };
  const testBed = createAddEffectTestBed({
    initialState: { settings: { isHydrated } },
    services: { preferenceService, tolgee: { changeLanguage: vi.fn() } },
  });
  applySettingsEffects(testBed.addEffect);
  return { testBed, preferenceService };
}

describe('settings effects - generic persistence', () => {
  it('persists a generic preference when hydrated', async () => {
    const { testBed, preferenceService } = makeTestBed(true);
    await testBed.dispatchHandled(setColorSchemeSeed('#abcdef'));
    expect(preferenceService.setPreference).toHaveBeenCalledWith('colorSchemeSeed', '#abcdef');
  });

  it('does not persist before hydration', async () => {
    const { testBed, preferenceService } = makeTestBed(false);
    await testBed.dispatchHandled(setColorSchemeSeed('#abcdef'));
    expect(preferenceService.setPreference).not.toHaveBeenCalled();
  });

  it('routes a persist:false key through its bespoke effect, not the generic one', async () => {
    const { testBed, preferenceService } = makeTestBed(true);
    await testBed.dispatchHandled(setPreferredLanguage('fr'));
    expect(preferenceService.setPreferredLanguage).toHaveBeenCalledWith('fr');
    expect(preferenceService.setPreference).not.toHaveBeenCalled();
  });
});
