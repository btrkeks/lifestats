import { FileExportService } from '@/services/file-export-service';
import { FilePickerService } from '@/services/file-picker-service';
import { KeyValueStore } from '@/services/key-value-store';
import { Logger } from '@/services/logger';
import { NotificationService } from '@/services/notification-service';
import { PreferenceService } from '@/services/preference-service';
import { SessionService } from '@/services/session-service';
import { getTolgee } from '@/services/tolgee';
import { WorkoutWorker } from '@/services/workout-worker';
import { RootState } from '@/store';
import { Store } from '@reduxjs/toolkit';
import { DatabaseMigrationService } from './database-migration-service';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';
import { SQLiteDatabase } from 'expo-sqlite';
import { DatabaseImportService } from '@/services/database-import-service';

export type Services = ReturnType<typeof createServices>;

export function createServices(store: Store<RootState>, db: ExpoSQLiteDatabase, expoDb: SQLiteDatabase) {
  const logger = new Logger();
  const keyValueStore = new KeyValueStore();
  const sessionService = new SessionService(db, store.getState);
  const notificationService = new NotificationService(store.getState, store.dispatch);
  const fileExportService = new FileExportService();
  const filePickerService = new FilePickerService();
  const preferenceService = new PreferenceService(keyValueStore);
  const tolgee = getTolgee(preferenceService);
  const workoutWorkerService = new WorkoutWorker(store.dispatch, store.getState, tolgee);
  const databaseMigrationService = new DatabaseMigrationService(
    db,
    logger,
    new DatabaseImportService(db, keyValueStore, preferenceService),
  );

  return {
    logger,
    keyValueStore,
    sessionService,
    notificationService,
    fileExportService,
    filePickerService,
    preferenceService,
    workoutWorkerService,
    tolgee,
    db,
    expoDb,
    databaseMigrationService,
  };
}
