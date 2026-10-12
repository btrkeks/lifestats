import { BackupData } from '@/models/backup';
import {
  CsvImportOptions,
  programFromNormalized,
  programIdFromCsvImport,
  sessionsFromNormalized,
} from '@/services/csv-import/csv-to-sessions';
import { fitNotesRowsToNormalized, parseFitNotesCsv } from '@/services/csv-import/fitnotes-csv';
import { parseStrongCsv, strongRowsToNormalized } from '@/services/csv-import/strong-csv';
import { parseStrongLiftsCsv, strongLiftsRowsToNormalized } from '@/services/csv-import/stronglifts-csv';

function decodeCsvBytes(contentBytes: Uint8Array): string {
  try {
    return new TextDecoder('utf-8').decode(contentBytes);
  } catch {
    throw new Error('Could not read the selected file as text');
  }
}

/**
 * FitNotes CSV column layout → BackupData for importBackupData.
 * Throws Error with a user-facing message on parse/map failure.
 */
export function getImportForFitNotes(contentBytes: Uint8Array, options?: CsvImportOptions): BackupData {
  const parsed = parseFitNotesCsv(decodeCsvBytes(contentBytes));
  if (!parsed.ok) {
    throw new Error(parsed.error);
  }
  const sessions = sessionsFromNormalized(fitNotesRowsToNormalized(parsed.rows, options));
  if (sessions.length === 0) {
    throw new Error('No weighted sets found to import');
  }
  return {
    workouts: sessions,
    programs: {},
  };
}

/**
 * StrongLifts CSV export → BackupData for importBackupData.
 * Throws Error with a user-facing message on parse/map failure.
 */
export function getImportForStrongLifts(contentBytes: Uint8Array, options?: CsvImportOptions): BackupData {
  const parsed = parseStrongLiftsCsv(decodeCsvBytes(contentBytes));
  if (!parsed.ok) {
    throw new Error(parsed.error);
  }
  const sessions = sessionsFromNormalized(
    strongLiftsRowsToNormalized(parsed.rows, parsed.weightUnit, parsed.bodyWeightUnit, options),
  );
  if (sessions.length === 0) {
    throw new Error('No weighted sets found to import');
  }
  return {
    workouts: sessions,
    programs: {},
  };
}

const STRONG_PLAN_NAME = 'Strong';

/**
 * Strong CSV export → BackupData for importBackupData, plus a "Strong" plan built from the workouts
 * that repeat (Strong's templates). Throws Error with a user-facing message on parse/map failure.
 */
export function getImportForStrong(contentBytes: Uint8Array, options?: CsvImportOptions): BackupData {
  const parsed = parseStrongCsv(decodeCsvBytes(contentBytes));
  if (!parsed.ok) {
    throw new Error(parsed.error);
  }
  const normalized = strongRowsToNormalized(parsed.rows, parsed.weightUnit, options);
  const sessions = sessionsFromNormalized(normalized);
  if (sessions.length === 0) {
    throw new Error('No weighted sets found to import');
  }
  const program = programFromNormalized(STRONG_PLAN_NAME, normalized);
  return {
    workouts: sessions,
    programs: program ? { [programIdFromCsvImport(STRONG_PLAN_NAME)]: program } : {},
  };
}
