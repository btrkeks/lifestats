import { importBackupData, importFromExternal, ExternalImportFormat } from '@/store/settings';
import { addImportExternalEffects } from '@/store/settings/import-external-effects';
import { getImportForFitNotes, getImportForStrongLifts } from '@/services/csv-import';
import { programIdFromCsvImport } from '@/services/csv-import/csv-to-sessions';
import { csvImportFixtureBytes } from '@/services/csv-import/__test__/fixtures';
import { STRONG_EXPORT_KG } from '@/services/csv-import/__test__/strong-fixture';
import { fetchUpcomingSessions, setActivePlan } from '@/store/program';
import { ProgramBlueprint } from '@/models/blueprint-models';
import { BuiltInPrograms } from '@/models/built-in-programs';
import { LocalDate } from '@js-joda/core';
import { createAddEffectTestBed } from '@/utils/__test__/add-effect-testbed';
import { createTestDb } from '@/utils/__test__/test-db';
import { describe, expect, it, vi } from 'vitest';
import { showSnackbar } from '@/store/app';
import { setStatsIsDirty } from '@/store/stats';
import { Session } from '@/models/session-models';

type BedOpts = {
  format: ExternalImportFormat;
  bytes: Uint8Array;
  sessions?: Session[];
  imperial?: boolean;
  program?: { activePlanId: string; savedPrograms: Record<string, ProgramBlueprint> };
  tolgee?: (key: string, params?: { count?: number; error?: string }) => string;
};

async function makeExternalImportBed({
  format: _format,
  bytes,
  sessions = [],
  imperial = false,
  program = { activePlanId: '', savedPrograms: {} },
  tolgee,
}: BedOpts) {
  const defaultTolgee = (key: string, params?: { count?: number; error?: string }) => {
    if (key === 'backup.import_from_other_apps.complete.message') {
      return `Imported ${params?.count} workout(s)`;
    }
    if (key === 'backup.import_from_other_apps.already_imported.message') {
      return 'Those workouts are already in History';
    }
    if (key === 'backup.import_from_other_apps.failed.message') {
      return `Could not import: ${params?.error}`;
    }
    return key;
  };

  const testBed = createAddEffectTestBed({
    initialState: {
      settings: { useImperialUnits: imperial },
      program,
    },
    services: {
      db: await createTestDb(sessions),
      filePickerService: {
        pickFile: vi.fn().mockResolvedValue({
          name: 'import.csv',
          bytes,
        }),
      },
      tolgee: {
        t: tolgee ?? defaultTolgee,
      },
    },
  });
  addImportExternalEffects(testBed.addEffect);
  return testBed;
}

describe('import-external-effects', () => {
  it('picks a FitNotes CSV and dispatches importBackupData', async () => {
    const testBed = await makeExternalImportBed({
      format: 'FitNotes',
      bytes: csvImportFixtureBytes('fitnotes-android-export-kgs.csv'),
    });
    await testBed.dispatchHandled(importFromExternal({ format: 'FitNotes' }));

    const imported = testBed.getDispatchedAction(importBackupData);
    expect(imported.payload.workouts).toHaveLength(2);
    expect(imported.payload.workouts[0]!.date.toString()).toBe('2026-08-04');
    expect(imported.payload.workouts[0]!.recordedExercises).toHaveLength(5);
    expect(imported.payload.programs).toEqual({});
    expect(imported.payload.successMessage).toBe('Imported 2 workout(s)');
    expect(testBed.getDispatchedAction(setStatsIsDirty).payload).toBe(true);
  });

  it.each([
    {
      name: 'FitNotes',
      format: 'FitNotes' as const,
      bytes: () => csvImportFixtureBytes('fitnotes-android-export-kgs.csv'),
      backup: () => getImportForFitNotes(csvImportFixtureBytes('fitnotes-android-export-kgs.csv')),
    },
    {
      name: 'StrongLifts',
      format: 'StrongLifts' as const,
      bytes: () => csvImportFixtureBytes('stronglifts-export-kg.csv'),
      backup: () => getImportForStrongLifts(csvImportFixtureBytes('stronglifts-export-kg.csv')),
    },
  ] as const)('skips import when all $name sessions already exist', async ({ format, bytes, backup }) => {
    const existing = backup().workouts;
    const testBed = await makeExternalImportBed({ format, bytes: bytes(), sessions: existing });
    await testBed.dispatchHandled(importFromExternal({ format }));

    expect(() => testBed.getDispatchedAction(importBackupData)).toThrow();
    expect(() => testBed.getDispatchedAction(setStatsIsDirty)).toThrow();
    expect(testBed.getDispatchedAction(showSnackbar).payload.text).toBe('Those workouts are already in History');
  });

  it('skips a FitNotes day already in History even when sets changed', async () => {
    const original = `Date,Exercise,Category,Weight,Weight Unit,Reps,Distance,Distance Unit,Time,Comment
2026-08-08,Bench Press,Chest,60,kgs,8,,,,
`;
    const changedSets = `Date,Exercise,Category,Weight,Weight Unit,Reps,Distance,Distance Unit,Time,Comment
2026-08-08,Bench Press,Chest,65,kgs,8,,,,
`;
    const existing = getImportForFitNotes(new TextEncoder().encode(original)).workouts;
    const testBed = await makeExternalImportBed({
      format: 'FitNotes',
      bytes: new TextEncoder().encode(changedSets),
      sessions: existing,
    });
    await testBed.dispatchHandled(importFromExternal({ format: 'FitNotes' }));

    expect(() => testBed.getDispatchedAction(importBackupData)).toThrow();
    expect(testBed.getDispatchedAction(showSnackbar).payload.text).toBe('Those workouts are already in History');
  });

  it('imports only sessions not already in History', async () => {
    const all = getImportForFitNotes(csvImportFixtureBytes('fitnotes-android-export-kgs.csv'));
    const firstOnly = [all.workouts[0]!];

    const testBed = await makeExternalImportBed({
      format: 'FitNotes',
      bytes: csvImportFixtureBytes('fitnotes-android-export-kgs.csv'),
      sessions: firstOnly,
    });
    await testBed.dispatchHandled(importFromExternal({ format: 'FitNotes' }));

    const imported = testBed.getDispatchedAction(importBackupData);
    expect(imported.payload.workouts).toHaveLength(1);
    expect(imported.payload.workouts[0]!.id).toBe(all.workouts[1]!.id);
    expect(imported.payload.workouts[0]!.date.toString()).toBe('2026-08-05');
    expect(imported.payload.successMessage).toBe('Imported 1 workout(s)');
  });

  it('shows an error snackbar for invalid CSV and does not import', async () => {
    const testBed = await makeExternalImportBed({
      format: 'FitNotes',
      bytes: new TextEncoder().encode('not,a,valid,file\n1,2,3,4\n'),
    });
    await testBed.dispatchHandled(importFromExternal({ format: 'FitNotes' }));

    expect(() => testBed.getDispatchedAction(importBackupData)).toThrow();
    expect(testBed.getDispatchedAction(showSnackbar).payload.text).toMatch(/Could not import:/);
  });

  it('no-ops when the user cancels the picker', async () => {
    const testBed = createAddEffectTestBed({
      services: {
        filePickerService: {
          pickFile: vi.fn().mockResolvedValue(undefined),
        },
        tolgee: { t: (s: string) => s },
      },
    });

    addImportExternalEffects(testBed.addEffect);
    await testBed.dispatchHandled(importFromExternal({ format: 'FitNotes' }));

    expect(() => testBed.getDispatchedAction(importBackupData)).toThrow();
    expect(() => testBed.getDispatchedAction(showSnackbar)).toThrow();
  });

  it('picks a StrongLifts CSV and dispatches importBackupData', async () => {
    const testBed = await makeExternalImportBed({
      format: 'StrongLifts',
      bytes: csvImportFixtureBytes('stronglifts-export-kg.csv'),
    });
    await testBed.dispatchHandled(importFromExternal({ format: 'StrongLifts' }));

    const imported = testBed.getDispatchedAction(importBackupData);
    expect(imported.payload.workouts).toHaveLength(5);
    expect(imported.payload.workouts[0]!.blueprint.name).toBe('Workout A');
    expect(imported.payload.workouts[0]!.recordedExercises).toHaveLength(3);
    expect(imported.payload.programs).toEqual({});
    expect(imported.payload.successMessage).toBe('Imported 5 workout(s)');
    expect(testBed.getDispatchedAction(setStatsIsDirty).payload).toBe(true);
  });

  describe('Strong plan activation', () => {
    const strongPlanId = programIdFromCsvImport('Strong');
    const [builtInId, builtIn] = Object.entries(BuiltInPrograms)[0]!;

    async function importStrongWith(program: BedOpts['program']) {
      const testBed = await makeExternalImportBed({
        format: 'Strong',
        bytes: new TextEncoder().encode(STRONG_EXPORT_KG),
        program,
      });
      await testBed.dispatchHandled(importFromExternal({ format: 'Strong' }));
      expect(Object.keys(testBed.getDispatchedAction(importBackupData).payload.programs)).toEqual([strongPlanId]);
      return testBed;
    }

    it('activates the imported plan when the active plan has no workouts', async () => {
      const testBed = await importStrongWith({
        activePlanId: 'mine',
        savedPrograms: { mine: new ProgramBlueprint('My Plan', [], LocalDate.of(2026, 1, 1)) },
      });
      expect(testBed.getDispatchedAction(setActivePlan).payload).toEqual({ activePlanId: strongPlanId });
      // The Workout tab shows the imported plan without being revisited.
      expect(() => testBed.getDispatchedAction(fetchUpcomingSessions)).not.toThrow();
    });

    it('activates the imported plan when the active plan is an untouched built-in', async () => {
      const testBed = await importStrongWith({ activePlanId: builtInId, savedPrograms: BuiltInPrograms });
      expect(testBed.getDispatchedAction(setActivePlan).payload).toEqual({ activePlanId: strongPlanId });
    });

    it('keeps a Strong plan that is already saved, edits included', async () => {
      const edited = new ProgramBlueprint('Strong, my way', [], LocalDate.of(2026, 2, 1));
      const testBed = await makeExternalImportBed({
        format: 'Strong',
        bytes: new TextEncoder().encode(STRONG_EXPORT_KG),
        program: { activePlanId: strongPlanId, savedPrograms: { [strongPlanId]: edited } },
      });
      await testBed.dispatchHandled(importFromExternal({ format: 'Strong' }));

      const imported = testBed.getDispatchedAction(importBackupData).payload;
      expect(imported.workouts).toHaveLength(5);
      expect(imported.programs).toEqual({});
      expect(() => testBed.getDispatchedAction(setActivePlan)).toThrow();
    });

    it('leaves an edited plan active', async () => {
      const testBed = await importStrongWith({
        activePlanId: builtInId,
        savedPrograms: { [builtInId]: builtIn.withName('Mine') },
      });
      expect(() => testBed.getDispatchedAction(setActivePlan)).toThrow();
    });
  });
});
