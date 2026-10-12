import { copyLogs, initializeAppStateSlice, setCurrentSnackbar, setIsHydrated, showSnackbar } from '@/store/app';
import { AddEffectFn } from '@/store/store';
import { sleep } from '@/utils/sleep';
import { initializeSettingsStateSlice } from '../settings';
import { initializeProgramStateSlice } from '../program';
import { setStringAsync } from 'expo-clipboard';

export function applyAppEffects(addEffect: AddEffectFn) {
  addEffect(
    initializeAppStateSlice,
    async (_, { cancelActiveListeners, dispatch, extra: { databaseMigrationService } }) => {
      cancelActiveListeners();
      await databaseMigrationService.migrate();
      dispatch(initializeSettingsStateSlice());
      dispatch(initializeProgramStateSlice());
      dispatch(setIsHydrated(true));
    },
  );

  addEffect(showSnackbar, async (action, { dispatch, getState }) => {
    dispatch(setCurrentSnackbar(action.payload));
    await sleep(action.payload.duration ?? 5000);
    if (getState().app.currentSnackbar === action.payload) {
      dispatch(setCurrentSnackbar(undefined));
    }
  });

  addEffect(copyLogs, async (_, { extra: { logger } }) => {
    const logs = await logger.getLogsAsString();
    await setStringAsync(logs);
  });
}
