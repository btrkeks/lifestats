import { LiftLog } from '@/gen/proto';
import { BackupData } from '@/models/backup';
import type { ExternalImportFormat } from '@/services/csv-import';
import { WeightUnit } from '@/models/weight';
import { createAction, createSlice, PayloadAction } from '@reduxjs/toolkit';
import { SQLiteDatabase } from 'expo-sqlite';
import {
  isPreferenceAction,
  preferenceKeys,
  preferenceRegistry,
  preferenceSetters,
  PrefKey,
  PrefValue,
} from './registry';

export type { ColorSchemeSeed, ThemeMode } from './codecs';
export type { ExternalImportFormat };

type PreferenceState = { [K in PrefKey]: PrefValue<K> };
type SettingsState = PreferenceState & { isHydrated: boolean };

const initialState: SettingsState = {
  ...(Object.fromEntries(preferenceKeys.map((key) => [key, preferenceRegistry[key].default])) as PreferenceState),
  isHydrated: false,
};

/** What the exported selectors need off the root state. */
export type SettingsRootState = { settings: SettingsState };
const settingsSlice = createSlice({
  name: 'settings',
  initialState,
  reducers: {
    setIsHydrated(state, action: PayloadAction<boolean>) {
      state.isHydrated = action.payload;
    },
  },
  extraReducers: (builder) => {
    // Every generated setter carries its key in `meta`, so one matcher applies
    // them all - no per-key reducer.
    builder.addMatcher(isPreferenceAction, (state, action) => {
      (state as Record<PrefKey, unknown>)[action.meta.prefKey] = action.payload;
    });
  },
  selectors: {
    selectPreferredWeightUnit: (state): WeightUnit => (state.useImperialUnits ? 'pounds' : 'kilograms'),
  },
});
export const initializeSettingsStateSlice = createAction('initializeSettingsStateSlice');
export type PlaintextExportFormat = 'CSV' | 'JSON';

export const importData = createAction('importData');
export const importDataSql = createAction<{ db: SQLiteDatabase }>('importDataSql');
export const importDataProto = createAction<{
  dao: LiftLog.Ui.Models.ExportedDataDao.ExportedDataDaoV2;
}>('importDataProto');
export type ImportBackupDataPayload = BackupData & {
  successMessage: string;
};
export const importBackupData = createAction<ImportBackupDataPayload>('importBackupData');
export const exportData = createAction('exportData');

export const exportPlainText = createAction<{ format: PlaintextExportFormat }>('exportPlainText');

/** Pick a third-party export file and merge history via importBackupData. */
export const importFromExternal = createAction<{ format: ExternalImportFormat }>('importFromExternal');

export const { setIsHydrated } = settingsSlice.actions;

export const {
  setUseImperialUnits,
  setShowBodyweight,
  setRestNotifications,
  setRestTimersEnabled,
  setColorSchemeSeed,
  setFirstDayOfWeek,
  setPlansSortOrder,
  setPreferredLanguage,
  setNotesExpandedByDefault,
  setKeepScreenAwakeDuringWorkout,
  setShowPostWorkoutSummary,
  setTrueBlackDarkTheme,
  setThemeMode,
} = preferenceSetters;

export const { selectPreferredWeightUnit } = settingsSlice.selectors;

export const settingsReducer = settingsSlice.reducer;
