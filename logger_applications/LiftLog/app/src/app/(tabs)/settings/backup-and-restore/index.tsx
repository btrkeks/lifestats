import { SettingsPage } from '@/components/layout/settings-page';
import { SegmentedGroup, SegmentListFormElement } from '@/components/presentation/foundation/segmented-list';
import { SegmentedListLink } from '@/components/presentation/foundation/segmented-list-link';
import { exportData, importData } from '@/store/settings';
import { setStatsIsDirty } from '@/store/stats';

import { useTranslate } from '@tolgee/react';
import { useRouter } from 'expo-router';
import { useDispatch } from 'react-redux';

export default function BackupAndRestorePage() {
  const { t } = useTranslate();
  const dispatch = useDispatch();
  const { push } = useRouter();
  return (
    <SettingsPage title={t('backup.export_backup_restore.title')} caption={t('backup.export_backup_restore.subtitle')}>
      <SegmentedGroup>
        <SegmentListFormElement
          label={t('backup.backup_data.title')}
          supportingText={t('backup.backup_data.subtitle')}
          icon={'backup'}
          onPress={() => dispatch(exportData())}
        />
        <SegmentListFormElement
          label={t('backup.restore_data.title')}
          supportingText={t('backup.restore_data.subtitle')}
          icon={'history'}
          onPress={() => {
            dispatch(importData());
            dispatch(setStatsIsDirty(true));
          }}
        />
      </SegmentedGroup>

      <SegmentedGroup>
        <SegmentedListLink
          label={t('backup.import_from_other_apps.title')}
          supportingText={t('backup.import_from_other_apps.subtitle')}
          icon={'download'}
          onPress={() => push('/settings/backup-and-restore/import-from-other-apps')}
        />
        <SegmentedListLink
          label={t('backup.plaintext_export.title')}
          supportingText={t('backup.plaintext_export.subtitle')}
          icon={'description'}
          onPress={() => push('/settings/backup-and-restore/plain-text-export')}
        />
      </SegmentedGroup>
    </SettingsPage>
  );
}
