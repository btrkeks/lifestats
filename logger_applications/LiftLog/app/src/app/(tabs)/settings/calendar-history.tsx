import { SettingsPage } from '@/components/layout/settings-page';
import { FormRow } from '@/components/presentation/foundation/form-row';
import { IndeterminateProgress } from '@/components/presentation/foundation/indeterminate-progress';
import NativeButton from '@/components/presentation/foundation/native-button';
import { SegmentedGroup, SegmentListFormElement } from '@/components/presentation/foundation/segmented-list';
import { SegmentedListSwitch } from '@/components/presentation/foundation/segmented-list-switch';
import { useServices } from '@/components/smart/services-provider';
import {
  BackfillApplyResult,
  BackfillApplyRow,
  BackfillError,
  BackfillPreview,
  BackfillRequest,
  BackfillRow,
  isBackfillPreviewCurrent,
  parseBackfillRange,
} from '@/models/calendar-backfill';
import { CalendarTarget, Times } from '@/models/calendar-sync';
import { applyCalendarBackfill, defaultBackfillRange, previewCalendarBackfill } from '@/services/calendar-backfill';
import { RootState, useAppSelector } from '@/store';
import { calendarSessionsQueued, selectCalendarSyncTarget } from '@/store/calendar-sync';
import { DateTimeFormatter, Instant, ZoneId } from '@js-joda/core';
import { useTranslate } from '@tolgee/react';
import { useEffect, useRef, useState } from 'react';
import { HelperText, Text, TextInput } from 'react-native-paper';
import { useDispatch, useStore } from 'react-redux';

type ViewState =
  | { kind: 'idle' | 'loading' | 'error' }
  | { kind: 'ready'; preview: BackfillPreview; selectedIds: string[] }
  | { kind: 'applying'; preview: BackfillPreview }
  | { kind: 'results'; preview: BackfillPreview; result: BackfillApplyResult };

const statusKeys = {
  ready: 'settings.calendar_history.status.ready',
  'already-linked': 'settings.calendar_history.status.already_linked',
  missing: 'settings.calendar_history.status.missing',
  ambiguous: 'settings.calendar_history.status.ambiguous',
  unsupported: 'settings.calendar_history.status.unsupported',
  claimed: 'settings.calendar_history.status.claimed',
  'conflicting-workouts': 'settings.calendar_history.status.conflicting_workouts',
  error: 'settings.calendar_history.status.error',
} as const satisfies Record<BackfillRow['status'], string>;
const errorKeys = {
  'no-permission': 'settings.calendar_history.error.no_permission',
  'calendar-missing': 'settings.calendar_history.error.calendar_missing',
  unsupported: 'settings.calendar_history.error.unsupported',
  'provider-error': 'settings.calendar_history.error.provider_error',
} as const satisfies Record<BackfillError, string>;
const resultKeys = {
  queued: 'settings.calendar_history.result.queued',
  'already-linked': 'settings.calendar_history.status.already_linked',
  stale: 'settings.calendar_history.result.stale',
  claimed: 'settings.calendar_history.status.claimed',
  'not-ready': 'settings.calendar_history.result.not_ready',
  error: 'settings.calendar_history.status.error',
} as const satisfies Record<BackfillApplyRow['status'], string>;
const TIME = DateTimeFormatter.ofPattern('yyyy-MM-dd HH:mm:ss XXX');
const time = (ms: number) => Instant.ofEpochMilli(ms).atZone(ZoneId.SYSTEM).format(TIME);
const times = (span: Times) => `${time(span.startMs)} – ${time(span.endMs)}`;

export default function CalendarHistoryPage() {
  const { phoneCalendar } = useServices();
  const configured = useAppSelector(selectCalendarSyncTarget);
  const calendarName = useAppSelector((state: RootState) => state.settings.calendarSyncCalendar?.displayName ?? '');
  const hydrated = useAppSelector((state: RootState) => state.storedSessions.isHydrated);
  const target = phoneCalendar.supported && hydrated ? configured : undefined;
  return (
    <CalendarHistoryForm key={JSON.stringify({ target, calendarName })} target={target} calendarName={calendarName} />
  );
}

function CalendarHistoryForm({ target, calendarName }: { target: CalendarTarget | undefined; calendarName: string }) {
  const { t } = useTranslate();
  const { db, phoneCalendar, logger } = useServices();
  const dispatch = useDispatch();
  const store = useStore<RootState>();
  const [rangeText, setRangeText] = useState(() => {
    const range = defaultBackfillRange(db);
    return { from: range.from.toString(), to: range.to.toString() };
  });
  const liveRange = useRef(rangeText);
  const operation = useRef({ revision: 0 });
  const [state, setState] = useState<ViewState>({ kind: 'idle' });
  const range = parseBackfillRange(rangeText.from, rangeText.to);
  const request = target && range ? { target, range } : undefined;

  useEffect(() => {
    const current = operation.current;
    return () => {
      ++current.revision;
    };
  }, []);

  const view =
    'preview' in state && !isBackfillPreviewCurrent(state.preview, request) ? { kind: 'idle' as const } : state;
  const busy = view.kind === 'loading' || view.kind === 'applying';

  function changeRange(next: typeof rangeText) {
    liveRange.current = next;
    ++operation.current.revision;
    setRangeText(next);
    setState({ kind: 'idle' });
  }

  function liveRequest(): BackfillRequest | undefined {
    const current = store.getState();
    const target =
      phoneCalendar.supported && current.storedSessions.isHydrated ? selectCalendarSyncTarget(current) : undefined;
    const range = parseBackfillRange(liveRange.current.from, liveRange.current.to);
    return target && range ? { target, range } : undefined;
  }

  async function preview() {
    if (!request) return;
    const id = ++operation.current.revision;
    setState({ kind: 'loading' });
    try {
      const result = await previewCalendarBackfill({ db, calendar: phoneCalendar }, request);
      if (id === operation.current.revision && isBackfillPreviewCurrent(result, liveRequest())) {
        setState({ kind: 'ready', preview: result, selectedIds: [] });
      }
    } catch (error) {
      logger.warn('Calendar history preview failed', { error });
      if (id === operation.current.revision) setState({ kind: 'error' });
    }
  }

  async function apply() {
    if (view.kind !== 'ready' || !view.selectedIds.length) return;
    const { preview, selectedIds } = view;
    const id = ++operation.current.revision;
    setState({ kind: 'applying', preview });
    try {
      const result = await applyCalendarBackfill(
        { db, calendar: phoneCalendar },
        {
          preview,
          selectedIds,
          getRequest: () => (id === operation.current.revision ? liveRequest() : undefined),
        },
      );
      if (result.queued) dispatch(calendarSessionsQueued());
      if (id === operation.current.revision) setState({ kind: 'results', preview, result });
    } catch (error) {
      logger.warn('Calendar history apply failed', { error });
      if (id === operation.current.revision) setState({ kind: 'error' });
    }
  }

  const ready = view.kind === 'ready' ? view.preview.rows.filter((row) => row.status === 'ready') : [];
  const selected = view.kind === 'ready' ? view.selectedIds : [];
  const counts =
    view.kind === 'ready'
      ? Object.entries(statusKeys).flatMap(([status, key]) => {
          const count = view.preview.rows.filter((row) => row.status === status).length;
          return count ? [`${t(key)}: ${count}`] : [];
        })
      : [];

  return (
    <SettingsPage title={t('settings.calendar_history.title')} caption={t('settings.calendar_history.explanation')}>
      {target ? (
        <Text>
          {t('settings.calendar_history.target', {
            title: target.title,
            account: target.accountName,
            calendar: calendarName,
          })}
        </Text>
      ) : (
        <Text>{t('settings.calendar_history.configure')}</Text>
      )}
      <FormRow noGap>
        <TextInput
          testID="calendarHistoryFrom"
          mode="outlined"
          label={t('settings.calendar_history.from')}
          value={rangeText.from}
          autoCapitalize="none"
          autoCorrect={false}
          disabled={view.kind === 'applying'}
          onChangeText={(from) => changeRange({ ...rangeText, from })}
          error={!range}
        />
        <TextInput
          testID="calendarHistoryTo"
          mode="outlined"
          label={t('settings.calendar_history.to')}
          value={rangeText.to}
          autoCapitalize="none"
          autoCorrect={false}
          disabled={view.kind === 'applying'}
          onChangeText={(to) => changeRange({ ...rangeText, to })}
          error={!range}
        />
        {!range ? <HelperText type="error">{t('settings.calendar_history.invalid_range')}</HelperText> : undefined}
      </FormRow>
      <NativeButton
        label={t('settings.calendar_history.preview')}
        variant="tonal"
        disabled={!request || busy}
        onPress={() => void preview()}
      />
      {busy ? (
        <>
          <IndeterminateProgress />
          <Text>
            {t(view.kind === 'applying' ? 'settings.calendar_history.applying' : 'settings.calendar_history.loading')}
          </Text>
        </>
      ) : undefined}
      {view.kind === 'error' ? <Text>{t('settings.calendar_history.error.provider_error')}</Text> : undefined}
      {view.kind === 'ready' ? (
        <>
          <Text>
            {t('settings.calendar_history.summary', {
              total: view.preview.rows.length,
              ready: ready.length,
              selected: selected.length,
            })}
          </Text>
          <Text>{counts.join('\n')}</Text>
          <FormRow>
            <NativeButton
              label={t('settings.calendar_history.select_all')}
              variant="text"
              disabled={!ready.length}
              onPress={() =>
                setState((current) =>
                  current.kind === 'ready' ? { ...current, selectedIds: ready.map((row) => row.sessionId) } : current,
                )
              }
            />
            <NativeButton
              label={t('settings.calendar_history.clear')}
              variant="text"
              disabled={!selected.length}
              onPress={() =>
                setState((current) => (current.kind === 'ready' ? { ...current, selectedIds: [] } : current))
              }
            />
            <NativeButton
              label={t('settings.calendar_history.apply', { count: selected.length })}
              disabled={!selected.length}
              onPress={() => void apply()}
            />
          </FormRow>
          <SegmentedGroup>
            {view.preview.rows.map((row) => {
              const label = `${row.date.toString()} · ${row.name}`;
              const status = row.status === 'error' ? t(errorKeys[row.reason]) : t(statusKeys[row.status]);
              const supportingText =
                'candidate' in row
                  ? `${status}\n${t('settings.calendar_history.change', { old: times(row.candidate.current), next: times(row.desired) })}`
                  : status;
              return row.status === 'ready' ? (
                <SegmentedListSwitch
                  key={row.sessionId}
                  label={label}
                  supportingText={supportingText}
                  value={selected.includes(row.sessionId)}
                  onValueChange={(value) =>
                    setState((current) =>
                      current.kind === 'ready'
                        ? {
                            ...current,
                            selectedIds: value
                              ? [...new Set([...current.selectedIds, row.sessionId])]
                              : current.selectedIds.filter((id) => id !== row.sessionId),
                          }
                        : current,
                    )
                  }
                />
              ) : (
                <SegmentListFormElement key={row.sessionId} label={label} supportingText={supportingText} />
              );
            })}
          </SegmentedGroup>
        </>
      ) : undefined}
      {view.kind === 'results' ? (
        <>
          <Text>
            {t('settings.calendar_history.results', {
              queued: view.result.queued,
              skipped: view.result.rows.length - view.result.queued,
            })}
          </Text>
          <Text>{t('settings.calendar_history.delivery')}</Text>
          <SegmentedGroup>
            {view.result.rows.map((row) => {
              const workout = view.preview.rows.find((entry) => entry.sessionId === row.sessionId);
              return (
                <SegmentListFormElement
                  key={row.sessionId}
                  label={workout ? `${workout.date.toString()} · ${workout.name}` : row.sessionId}
                  supportingText={row.status === 'error' ? t(errorKeys[row.reason]) : t(resultKeys[row.status])}
                />
              );
            })}
          </SegmentedGroup>
        </>
      ) : undefined}
    </SettingsPage>
  );
}
