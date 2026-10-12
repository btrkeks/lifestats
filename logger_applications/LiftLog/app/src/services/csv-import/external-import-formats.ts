import { BackupData } from '@/models/backup';
import { CsvImportOptions } from '@/services/csv-import/csv-to-sessions';
import { getImportForFitNotes, getImportForStrong, getImportForStrongLifts } from '@/services/csv-import/importers';
import type { TranslationKey } from '@tolgee/web';

export type ExternalImportFormat = 'Strong' | 'FitNotes' | 'StrongLifts';

export type ExternalImportFormatEntry = {
  id: ExternalImportFormat;
  labelKey: TranslationKey;
  import: (bytes: Uint8Array, opts?: CsvImportOptions) => BackupData;
};

export const EXTERNAL_IMPORT_FORMATS: readonly ExternalImportFormatEntry[] = [
  {
    id: 'Strong',
    labelKey: 'backup.import_from_other_apps.format.strong',
    import: getImportForStrong,
  },
  {
    id: 'FitNotes',
    labelKey: 'backup.import_from_other_apps.format.fitnotes',
    import: getImportForFitNotes,
  },
  {
    id: 'StrongLifts',
    labelKey: 'backup.import_from_other_apps.format.stronglifts',
    import: getImportForStrongLifts,
  },
];

const IMPORTERS: Record<ExternalImportFormat, (bytes: Uint8Array, opts?: CsvImportOptions) => BackupData> = {
  Strong: getImportForStrong,
  FitNotes: getImportForFitNotes,
  StrongLifts: getImportForStrongLifts,
};

export function getExternalImporter(
  format: ExternalImportFormat,
): (bytes: Uint8Array, opts?: CsvImportOptions) => BackupData {
  return IMPORTERS[format];
}
