import { SettingsPage } from '@/components/layout/settings-page';
import { SegmentedGroup, SegmentListFormElement } from '@/components/presentation/foundation/segmented-list';
import { SegmentedListLink } from '@/components/presentation/foundation/segmented-list-link';
import { useAppTheme } from '@/hooks/useAppTheme';
import { T, useTranslate } from '@tolgee/react';
import { Link, useRouter } from 'expo-router';
import { useState } from 'react';
import { Text, Dialog, Icon, Portal } from 'react-native-paper';
import Button from '@/components/presentation/foundation/button';
import * as Application from 'expo-application';
import { useDispatch } from 'react-redux';
import { copyLogs } from '@/store/app';

export default function SettingsPageIndex() {
  const { t } = useTranslate();
  const { colors } = useAppTheme();
  const { push } = useRouter();
  const [appInfoOpen, setAppInfoOpen] = useState(false);
  const dispatch = useDispatch();

  const doCopyLogs = () => {
    dispatch(copyLogs());
  };

  const appVersion = Application.nativeApplicationVersion ?? Application.nativeBuildVersion ?? 'Unknown';

  return (
    <SettingsPage title={t('settings.settings.title')}>
      <SegmentedGroup>
        <SegmentedListLink
          label={t('plan.manage.title')}
          icon={'assignment'}
          onPress={() => push('/settings/program-list')}
        />
        <SegmentedListLink
          label={t('exercise.manage.button')}
          icon={'directionsRun'}
          onPress={() => push('/settings/manage-exercises')}
        />
      </SegmentedGroup>

      <SegmentedGroup>
        <SegmentedListLink
          testID="appConfiguration"
          label={t('settings.app_configuration.title')}
          icon={'settings'}
          onPress={() => push('/settings/app-configuration')}
        />
        <SegmentedListLink
          testID="localization"
          label={t('settings.localisation.title')}
          icon={'language'}
          onPress={() => push('/settings/localization')}
        />
        <SegmentedListLink
          label={t('settings.notifications.title')}
          icon={'notifications'}
          onPress={() => push('/settings/notifications')}
        />
      </SegmentedGroup>

      <SegmentedGroup>
        <SegmentedListLink
          label={t('backup.export_backup_restore.title')}
          icon={'settingsBackupRestore'}
          onPress={() => push('/settings/backup-and-restore')}
        />
      </SegmentedGroup>

      <SegmentedGroup>
        <SegmentListFormElement label={t('settings.copy_logs.title')} icon={'terminal'} onPress={doCopyLogs} />
        <SegmentListFormElement
          label={t('settings.app_info.title')}
          icon={'info'}
          onPress={() => setAppInfoOpen(true)}
        />
      </SegmentedGroup>

      <Portal>
        <Dialog visible={appInfoOpen} onDismiss={() => setAppInfoOpen(false)}>
          <Dialog.Title>
            <T keyName="settings.app_info.title" />
          </Dialog.Title>
          <Dialog.Content>
            <Text>
              LiftLog is an entirely open source app, licensed under the AGPL-3.0 license. You can find the source code
              on{' '}
              <Link style={{ color: colors.primary, fontWeight: 'bold' }} href="https://github.com/LiamMorrow/LiftLog">
                <Icon size={16} source={'share'} color={colors.primary} />
                GitHub
              </Link>
              .
            </Text>
            <Text>LiftLog is currently version {appVersion}</Text>
          </Dialog.Content>
          <Dialog.Actions>
            <Button onPress={() => setAppInfoOpen(false)}>
              <T keyName="generic.close.button" />
            </Button>
          </Dialog.Actions>
        </Dialog>
      </Portal>
    </SettingsPage>
  );
}
