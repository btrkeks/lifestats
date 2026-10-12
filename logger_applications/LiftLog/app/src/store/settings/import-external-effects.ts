import { showSnackbar } from '@/store/app';
import { importBackupData, importFromExternal, selectPreferredWeightUnit } from '@/store/settings';
import { AddEffectFn, RootState } from '@/store/store';
import { setStatsIsDirty } from '@/store/stats';
import { getExternalImporter } from '@/services/csv-import';
import { readExistingSessionIds } from '@/db/sessions';
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
        if (newWorkouts.length === 0) {
          dispatch(
            showSnackbar({
              text: tolgee.t('backup.import_from_other_apps.already_imported.message'),
            }),
          );
          return;
        }

        const importedPlanId = Object.keys(backupData.programs)[0];
        const activateImportedPlan = importedPlanId !== undefined && activePlanIsReplaceable(getState());
        dispatch(
          importBackupData({
            workouts: newWorkouts,
            programs: backupData.programs,
            successMessage: tolgee.t('backup.import_from_other_apps.complete.message', {
              count: newWorkouts.length,
            }),
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

/** An imported plan may take over only an empty active plan or an untouched built-in one. */
function activePlanIsReplaceable(state: RootState): boolean {
  const { activePlanId, savedPrograms } = state.program;
  const active = savedPrograms[activePlanId];
  return !active || active.sessions.length === 0 || active.equals(BuiltInPrograms[activePlanId]);
}
