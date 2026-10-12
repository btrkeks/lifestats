import { SettingsPage } from '@/components/layout/settings-page';
import { FormRow } from '@/components/presentation/foundation/form-row';
import { SegmentedGroup, SegmentListFormElement } from '@/components/presentation/foundation/segmented-list';
import { SegmentedListSwitch } from '@/components/presentation/foundation/segmented-list-switch';
import { syncBaseUrl } from '@/services/lifestats-sync';
import { RootState, useAppSelector } from '@/store';
import { lifestatsSyncAllRequested } from '@/store/lifestats-sync';
import { setLifestatsSyncEnabled, setLifestatsSyncUrl } from '@/store/settings';
import { DateTimeFormatter, Instant, LocalTime, ZoneId } from '@js-joda/core';
import { useTranslate } from '@tolgee/react';
import { HelperText, Text, TextInput } from 'react-native-paper';
import { useDispatch } from 'react-redux';

const HOURS_MINUTES = DateTimeFormatter.ofPattern('HH:mm');

export default function LifestatsSyncPage() {
  const { t } = useTranslate();
  const dispatch = useDispatch();
  const { lifestatsSyncEnabled, lifestatsSyncUrl } = useAppSelector((state: RootState) => state.settings);
  const { pendingDays, pushing, lastPush } = useAppSelector((state: RootState) => state.lifestatsSync);
  const urlIsUsable = syncBaseUrl(lifestatsSyncUrl) !== undefined;

  const lastPushText = (() => {
    if (pushing) {
      return t('settings.lifestats_sync.status.syncing');
    }
    if (!lastPush) {
      return undefined;
    }
    const time = LocalTime.ofInstant(Instant.parse(lastPush.at), ZoneId.SYSTEM).format(HOURS_MINUTES);
    return lastPush.outcome.kind === 'delivered'
      ? t('settings.lifestats_sync.status.delivered', { time, count: lastPush.outcome.days })
      : t('settings.lifestats_sync.status.failed', { time, error: lastPush.outcome.error });
  })();

  return (
    <SettingsPage title={t('settings.lifestats_sync.title')} caption={t('settings.lifestats_sync.explanation')}>
      <FormRow noGap>
        <TextInput
          mode="outlined"
          label={t('settings.lifestats_sync.url.label')}
          placeholder="https://"
          autoCapitalize="none"
          autoCorrect={false}
          keyboardType="url"
          value={lifestatsSyncUrl ?? ''}
          error={!urlIsUsable}
          onChangeText={(url) => dispatch(setLifestatsSyncUrl(url))}
        />
        <HelperText type="error" visible={!urlIsUsable}>
          {t('settings.lifestats_sync.url.invalid')}
        </HelperText>
      </FormRow>
      <SegmentedGroup>
        <SegmentedListSwitch
          label={t('settings.lifestats_sync.enabled.label')}
          supportingText={t('settings.lifestats_sync.enabled.subtitle')}
          icon={'cloudUpload'}
          value={lifestatsSyncEnabled}
          onValueChange={(value) => dispatch(setLifestatsSyncEnabled(value))}
        />
        <SegmentListFormElement
          label={t('settings.lifestats_sync.sync_all.label')}
          supportingText={t('settings.lifestats_sync.sync_all.subtitle')}
          icon={'history'}
          onPress={() => dispatch(lifestatsSyncAllRequested())}
        />
      </SegmentedGroup>
      <Text variant="bodyMedium">{t('settings.lifestats_sync.status.pending', { count: pendingDays })}</Text>
      {lastPushText ? <Text variant="bodyMedium">{lastPushText}</Text> : undefined}
    </SettingsPage>
  );
}
