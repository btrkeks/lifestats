import { showSnackbar } from '@/store/app';
import { importBackupData, importFromExternal, selectPreferredWeightUnit } from '@/store/settings';
import { AddEffectFn, RootState } from '@/store/store';
import { setStatsIsDirty } from '@/store/stats';
import { getExternalImporter } from '@/services/csv-import';
import { retimedLikeImport } from '@/services/csv-import/csv-to-sessions';
import { readExistingSessionIds, readFinishedSessions } from '@/db/sessions';
import { Session } from '@/models/session-models';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { fetchUpcomingSessions, setActivePlan } from '@/store/program';
import { BuiltInPrograms } from '@/models/built-in-programs';

export function addImportExternalEffects(addEffect: AddEffectFn) {
  addEffect(
    importFromExternal,
    async ({ payload: { format } }, { dispatch, getState, extra: { db, filePickerService, tolgee, logger } }) => {
      const file = await filePickerService.pickFile();
      if (!file) {
        return;
      }

      const defaultWeightUnit = selectPreferredWeightUnit(getState());

      try {
        const backupData = getExternalImporter(format)(file.bytes, { defaultWeightUnit });

        // Grouping-key session ids: skip any workout already present so re-importing
        // the same day (FitNotes) or date+Workout slot (StrongLifts) is a no-op.
        const existingIds = readExistingSessionIds(
          db,
          backupData.workouts.map((w) => w.id),
        );
        const newWorkouts = backupData.workouts.filter((w) => !existingIds.has(w.id));
        const retimed =
          format === 'Strong'
            ? retimedStrongWorkouts(
                db,
                backupData.workouts.filter((w) => existingIds.has(w.id)),
              )
            : [];
        if (newWorkouts.length === 0 && retimed.length === 0) {
          dispatch(
            showSnackbar({
              text: tolgee.t('backup.import_from_other_apps.already_imported.message'),
            }),
          );
          return;
        }

        // A plan already saved under the import's id may carry the user's edits, so it is never replaced.
        const { savedPrograms } = getState().program;
        const newPrograms = Object.fromEntries(
          Object.entries(backupData.programs).filter(([id]) => !savedPrograms[id]),
        );
        const importedPlanId = Object.keys(newPrograms)[0];
        const activateImportedPlan = importedPlanId !== undefined && activePlanIsReplaceable(getState());
        dispatch(
          importBackupData({
            workouts: [...newWorkouts, ...retimed],
            programs: newPrograms,
            successMessage: retimed.length
              ? tolgee.t('backup.import_from_other_apps.complete_retimed.message', {
                  count: newWorkouts.length,
                  retimed: retimed.length,
                })
              : tolgee.t('backup.import_from_other_apps.complete.message', { count: newWorkouts.length }),
          }),
        );
        // importBackupData's effect upserts the plans synchronously within that dispatch, so the
        // imported plan is already saved when it becomes active here.
        if (activateImportedPlan) {
          dispatch(setActivePlan({ activePlanId: importedPlanId }));
        }
        // History merge only: full restore via importBackupData leaves stats dirty handling unchanged.
        dispatch(setStatsIsDirty(true));
        // New history changes the prefilled weights, and the active plan may have just changed.
        dispatch(fetchUpcomingSessions());
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        logger.warn('External import failed', { err, format });
        dispatch(
          showSnackbar({
            text: tolgee.t('backup.import_from_other_apps.failed.message', { error: message }),
          }),
        );
      }
    },
  );
}

/**
 * Older imports timed Strong sets one second apart, so those workouts lasted seconds. Re-importing the
 * export gives each already-imported, finished workout its real span and changes nothing else.
 */
function retimedStrongWorkouts(db: ExpoSQLiteDatabase, imported: Session[]): Session[] {
  const existingById = new Map(
    readFinishedSessions(
      db,
      imported.map((s) => s.id),
    ).map((s) => [s.id, s]),
  );
  return imported.flatMap((fresh) => {
    const existing = existingById.get(fresh.id);
    const retimed = existing && retimedLikeImport(existing, fresh);
    return retimed && !retimed.equals(existing) ? [retimed] : [];
  });
}

/** An imported plan may take over only an empty active plan or an untouched built-in one. */
function activePlanIsReplaceable(state: RootState): boolean {
  const { activePlanId, savedPrograms } = state.program;
  const active = savedPrograms[activePlanId];
  return !active || active.sessions.length === 0 || active.equals(BuiltInPrograms[activePlanId]);
}
