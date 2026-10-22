import { SettingsPage } from '@/components/layout/settings-page';
import { FormRow } from '@/components/presentation/foundation/form-row';
import { SegmentedGroup, SegmentListFormElement } from '@/components/presentation/foundation/segmented-list';
import { SegmentedListSelect } from '@/components/presentation/foundation/segmented-list-select';
import { SegmentedListSwitch } from '@/components/presentation/foundation/segmented-list-switch';
import { useServices } from '@/components/smart/services-provider';
import { CalendarAccess, CalendarChoice, CalendarOutcome } from '@/models/calendar-sync';
import { RootState, useAppSelector } from '@/store';
import { calendarRetryRequested } from '@/store/calendar-sync';
import { setCalendarSyncCalendar, setCalendarSyncEnabled, setCalendarSyncEventTitle } from '@/store/settings';
import { formatDate } from '@/utils/format-date';
import { useTranslate } from '@tolgee/react';
import { useFocusEffect } from 'expo-router';
import { useCallback, useState } from 'react';
import { AppState, Linking } from 'react-native';
import { HelperText, Text, TextInput } from 'react-native-paper';
import { useDispatch } from 'react-redux';

const outcomeKeys = {
  adjusted: 'settings.calendar_sync.outcome.adjusted',
  idle: 'settings.calendar_sync.outcome.idle',
  'no-match': 'settings.calendar_sync.outcome.no_match',
  ambiguous: 'settings.calendar_sync.outcome.ambiguous',
  claimed: 'settings.calendar_sync.outcome.claimed',
  unsupported: 'settings.calendar_sync.outcome.unsupported',
  'moved-date': 'settings.calendar_sync.outcome.moved_date',
  changed: 'settings.calendar_sync.outcome.changed',
  gone: 'settings.calendar_sync.outcome.gone',
  duplicate: 'settings.calendar_sync.outcome.duplicate',
  'calendar-missing': 'settings.calendar_sync.outcome.calendar_missing',
  unsynced: 'settings.calendar_sync.outcome.unsynced',
  error: 'settings.calendar_sync.outcome.error',
} as const satisfies Record<CalendarOutcome, string>;

const calendarLabel = (calendar: CalendarChoice) => `${calendar.displayName} · ${calendar.accountName}`;

export default function CalendarSyncPage() {
  const { t } = useTranslate();
  const dispatch = useDispatch();
  const { phoneCalendar } = useServices();
  const { calendarSyncEnabled, calendarSyncCalendar, calendarSyncEventTitle, preferredLanguage } = useAppSelector(
    (state: RootState) => state.settings,
  );
  const { pending, attention, running, noPermission } = useAppSelector((state: RootState) => state.calendarSync);
  const [access, setAccess] = useState<CalendarAccess>();
  const [calendars, setCalendars] = useState<CalendarChoice[]>([]);

  // useFocusEffect subscribes by callback identity, including renders caused by its own async reads.
  useFocusEffect(
    useCallback(() => {
      let focused = true;
      let refreshRevision = 0;
      const refresh = async () => {
        const revision = ++refreshRevision;
        const current = await phoneCalendar.access();
        const available = current === 'granted' ? await phoneCalendar.calendars() : [];
        if (focused && revision === refreshRevision) {
          setAccess(current);
          setCalendars(available);
        }
      };
      void refresh();
      const subscription = AppState.addEventListener('change', (state) => {
        if (state === 'active') void refresh();
      });
      return () => {
        focused = false;
        subscription.remove();
      };
    }, [phoneCalendar]),
  );

  const allowAccess = async () => {
    if (access === 'blocked') {
      await Linking.openSettings();
      return;
    }
    const granted = await phoneCalendar.requestAccess();
    setAccess(granted);
    if (granted === 'granted') {
      setCalendars(await phoneCalendar.calendars());
      dispatch(calendarRetryRequested());
    }
  };

  const titleIsUsable = !!calendarSyncEventTitle?.trim();
  const chosenIsListed = calendars.some((c) => c.id === calendarSyncCalendar?.id);
  const calendarOptions = [
    ...calendars.map((c) => ({ value: c.id, label: calendarLabel(c) })),
    ...(calendarSyncCalendar && !chosenIsListed
      ? [{ value: calendarSyncCalendar.id, label: calendarLabel(calendarSyncCalendar) }]
      : []),
    ...(calendarSyncCalendar
      ? []
      : [{ value: undefined, label: t('settings.calendar_sync.calendar.none'), disabledAndHidden: true as const }]),
  ];

  const statusText = running
    ? t('settings.calendar_sync.status.running')
    : noPermission && pending
      ? t('settings.calendar_sync.status.no_permission', { count: pending })
      : t('settings.calendar_sync.status.pending', { count: pending });

  return (
    <SettingsPage title={t('settings.calendar_sync.title')} caption={t('settings.calendar_sync.explanation')}>
      {access !== undefined && access !== 'granted' ? (
        <SegmentedGroup>
          <SegmentListFormElement
            label={t('settings.calendar_sync.access.label')}
            supportingText={t(
              access === 'blocked' ? 'settings.calendar_sync.access.blocked' : 'settings.calendar_sync.access.subtitle',
            )}
            icon={'vpnKey'}
            onPress={() => void allowAccess()}
          />
        </SegmentedGroup>
      ) : undefined}
      <SegmentedGroup>
        <SegmentedListSelect
          testID="calendarSyncCalendar"
          icon={'calendar'}
          label={t('settings.calendar_sync.calendar.label')}
          supportingText={t('settings.calendar_sync.calendar.subtitle')}
          value={calendarSyncCalendar?.id}
          options={calendarOptions}
          enabled={calendars.length > 0}
          onChange={(id) => {
            const chosen = calendars.find((c) => c.id === id);
            if (chosen) {
              dispatch(setCalendarSyncCalendar(chosen));
            }
          }}
        />
      </SegmentedGroup>
      <FormRow noGap>
        <TextInput
          mode="outlined"
          label={t('settings.calendar_sync.title_input.label')}
          autoCorrect={false}
          value={calendarSyncEventTitle ?? ''}
          error={!titleIsUsable}
          onChangeText={(title) => dispatch(setCalendarSyncEventTitle(title))}
        />
        <HelperText type={titleIsUsable ? 'info' : 'error'}>
          {t(
            titleIsUsable
              ? 'settings.calendar_sync.title_input.subtitle'
              : 'settings.calendar_sync.title_input.invalid',
          )}
        </HelperText>
      </FormRow>
      <SegmentedGroup>
        <SegmentedListSwitch
          testID="calendarSyncEnabled"
          label={t('settings.calendar_sync.enabled.label')}
          supportingText={t('settings.calendar_sync.enabled.subtitle')}
          icon={'event'}
          value={calendarSyncEnabled}
          disabled={!calendarSyncEnabled && (!calendarSyncCalendar || !titleIsUsable)}
          onValueChange={(value) => dispatch(setCalendarSyncEnabled(value))}
        />
      </SegmentedGroup>
      <Text variant="bodyMedium">{statusText}</Text>
      {attention.length ? (
        <>
          <Text variant="titleSmall">{t('settings.calendar_sync.attention.title')}</Text>
          <SegmentedGroup>
            {attention.map(({ sessionId, date, outcome }) => (
              <SegmentListFormElement
                key={sessionId}
                label={
                  date
                    ? formatDate(preferredLanguage, date, { weekday: 'short', day: 'numeric', month: 'short' })
                    : t('settings.calendar_sync.attention.no_date')
                }
                supportingText={t(outcomeKeys[outcome])}
              />
            ))}
            <SegmentListFormElement
              label={t('settings.calendar_sync.retry.label')}
              supportingText={t('settings.calendar_sync.retry.subtitle')}
              icon={'reload'}
              onPress={() => dispatch(calendarRetryRequested())}
            />
          </SegmentedGroup>
        </>
      ) : undefined}
    </SettingsPage>
  );
}
