import CardActions from '@/components/presentation/foundation/card-actions';
import CardList from '@/components/presentation/foundation/card-list';
import ConfirmationDialog from '@/components/presentation/foundation/confirmation-dialog';
import FullHeightScrollView from '@/components/layout/full-height-scroll-view';
import IconButton from '@/components/presentation/foundation/icon-button';
import NativeButton from '@/components/presentation/foundation/native-button';
import EditIcon from '@expo/material-symbols/edit.xml';
import { useFormatDate } from '@/hooks/useFormatDate';
import PlanMenu, { usePlanNavigation } from '@/components/smart/plan-menu';
import { Remote } from '@/components/presentation/foundation/remote';
import SessionSummary from '@/components/presentation/summary/session-summary';
import SessionSummaryTitle from '@/components/presentation/summary/session-summary-title';
import SplitCardControl from '@/components/presentation/foundation/split-card-control';
import { spacing, useAppTheme } from '@/hooks/useAppTheme';
import { Session } from '@/models/session-models';
import { useAppSelector, useAppSelectorWhenFocused } from '@/store';
import { deleteStoredSession, selectActiveSession } from '@/store/stored-sessions';
import { fetchUpcomingSessions, selectActiveProgram } from '@/store/program';
import { ChronoUnit, Instant, LocalDate } from '@js-joda/core';
import { T, useTranslate } from '@tolgee/react';
import { Stack, useFocusEffect, useRouter } from 'expo-router';
import { useState } from 'react';
import { View } from 'react-native';
import { Card, Icon as PaperIcon, Text, Tooltip } from 'react-native-paper';
import Button from '@/components/presentation/foundation/button';
import { useDispatch } from 'react-redux';
import { useStartWorkoutWithConfirmation } from '@/hooks/useStartWorkoutWithConfirmation';
import { useSessionsQuery } from '@/hooks/useSessionsQuery';
import { readLastFinishedByName } from '@/db/sessions';
import { upNextIndex } from '@/models/up-next';
import { SurfaceText } from '@/components/presentation/foundation/surface-text';

function ListUpcomingWorkouts({
  upcoming,
  startSession,
}: {
  upcoming: readonly Session[];
  startSession: (s: Session) => void;
}) {
  const plan = useAppSelector(selectActiveProgram);
  const { t } = useTranslate();
  const formatDate = useFormatDate();
  const currentSession = useAppSelectorWhenFocused(selectActiveSession);
  const planId = useAppSelector((x) => x.program.activePlanId);
  const { push } = useRouter();
  const dispatch = useDispatch();
  const [confirmDeleteSessionOpen, setConfirmDeleteSessionOpen] = useState(false);
  const lastDoneLabel = useLastDoneLabel();
  const names = plan.sessions.map((x) => x.name);
  const lastFinished = useSessionsQuery((db) => readLastFinishedByName(db, names), names.join('\n'));
  const upNext = lastFinished && upNextIndex(names, lastFinished, currentSession?.blueprint.name);
  const planIndexOf = (session: Session) => plan.sessions.findIndex((x) => x.equals(session.blueprint));
  const clearCurrentSession = () => {
    if (currentSession) {
      dispatch(deleteStoredSession(currentSession.id));
    }
    dispatch(fetchUpcomingSessions());
  };

  return (
    <View style={{ flex: 1, gap: spacing[2], paddingTop: spacing[4] }}>
      {currentSession && (
        <>
          <SectionHeader
            title={t('workout.current.title')}
            trailing={formatDate(currentSession.date, {
              year: currentSession.date.year() !== LocalDate.now().year() ? 'numeric' : undefined,
              day: 'numeric',
              weekday: 'long',
              month: 'long',
            })}
          />
          <Card mode="contained">
            <Card.Content>
              <SessionCardContent session={currentSession} />
            </Card.Content>
            <CardActions style={{ marginTop: spacing[2] }}>
              <Tooltip title={t('workout.clear_current.button')}>
                <IconButton
                  testID="clear-current-workout"
                  icon={'delete'}
                  mode="contained"
                  onPress={() => setConfirmDeleteSessionOpen(true)}
                />
              </Tooltip>
              <Button
                icon={'playCircle'}
                mode="contained"
                testID="resume-workout-button"
                onPress={() => startSession(currentSession)}
              >
                <T keyName="workout.resume.button" />
              </Button>
            </CardActions>
          </Card>
        </>
      )}
      {!!upcoming.length && <SectionHeader title={t('workout.upcoming.title')} trailing={plan.name} />}
      <CardList
        keySelector={(x) => x.id}
        cardType="contained"
        items={upcoming}
        emptyTemplate={currentSession ? undefined : <NoUpcomingWorkouts />}
        shouldHighlight={(session) => planIndexOf(session) === upNext}
        renderItemContent={(session) => {
          return (
            <Card.Content>
              <SessionCardContent
                session={session}
                lastDone={lastFinished && lastDoneLabel(lastFinished.get(session.blueprint.name))}
                isUpNext={planIndexOf(session) === upNext}
              />
            </Card.Content>
          );
        }}
        renderItemActions={(session) => {
          const sessionPlanIndex = planIndexOf(session);
          const handleEditPress = () => {
            push(`/plan-workout-editor/${planId}/${sessionPlanIndex}`);
          };
          return (
            <CardActions style={{ marginTop: spacing[2] }}>
              {sessionPlanIndex !== -1 ? (
                <IconButton icon={'edit'} mode="contained" onPress={handleEditPress} />
              ) : undefined}
              <Button
                mode="contained"
                icon={'playCircle'}
                testID="start-resume-workout-button"
                onPress={() => startSession(session)}
              >
                {session.isStarted ? <T keyName="workout.resume.button" /> : <T keyName="workout.start.button" />}
              </Button>
            </CardActions>
          );
        }}
      />
      <ConfirmationDialog
        headline={t('workout.clear_current.confirm.title')}
        textContent={t('workout.clear_current.confirm.body')}
        okText={t('generic.clear.button')}
        onOk={() => {
          clearCurrentSession();
          setConfirmDeleteSessionOpen(false);
        }}
        open={confirmDeleteSessionOpen}
        onCancel={() => setConfirmDeleteSessionOpen(false)}
      />
    </View>
  );
}

function SectionHeader({ title, trailing }: { title: string; trailing: string }) {
  const { colors } = useAppTheme();

  return (
    <View
      style={{
        flexDirection: 'row',
        alignItems: 'baseline',
        justifyContent: 'space-between',
        gap: spacing[4],
        marginTop: spacing[2],
      }}
    >
      <Text variant="titleMedium">{title}</Text>
      <Text variant="bodyMedium" numberOfLines={1} style={{ flexShrink: 1, color: colors.onSurfaceVariant }}>
        {trailing}
      </Text>
    </View>
  );
}

function NoUpcomingWorkouts() {
  const { t } = useTranslate();
  const { colors } = useAppTheme();
  const { choosePlan, editWorkouts } = usePlanNavigation();

  return (
    <View
      style={{
        flex: 1,
        alignItems: 'center',
        justifyContent: 'center',
        gap: spacing[8],
        paddingBottom: spacing[12],
        paddingHorizontal: spacing[4],
      }}
    >
      <View style={{ alignItems: 'center', gap: spacing[3] }}>
        <PaperIcon source="fitnessCenter" size={40} color={colors.onSurfaceVariant} />
        <Text variant="titleLarge" style={{ textAlign: 'center' }}>
          {t('plan.no_upcoming_workouts.title')}
        </Text>
        <Text variant="bodyMedium" style={{ textAlign: 'center', color: colors.onSurfaceVariant }}>
          {t('plan.no_upcoming_workouts.message')}
        </Text>
      </View>
      <View style={{ flexDirection: 'row', justifyContent: 'center', alignItems: 'center', gap: spacing[2] }}>
        <NativeButton
          variant="filled"
          icon={EditIcon}
          systemImage="pencil"
          label={t('workout.edit_workouts.button')}
          onPress={editWorkouts}
        />
        <NativeButton variant="text" label={t('plan.choose.button')} onPress={choosePlan} />
      </View>
    </View>
  );
}

function SessionCardContent({
  session,
  lastDone,
  isUpNext = false,
}: {
  session: Session;
  lastDone?: string;
  isUpNext?: boolean;
}) {
  return (
    <SplitCardControl
      titleContent={
        <SessionSummaryTitle
          session={session}
          byline={lastDone}
          bylineLeading={isUpNext ? <UpNextChip /> : undefined}
          color={isUpNext ? 'onSecondaryContainer' : undefined}
        />
      }
      mainContent={<SessionSummary session={session} isFilled={false} showWeight />}
    />
  );
}

function UpNextChip() {
  const { t } = useTranslate();
  const { colors } = useAppTheme();
  return (
    <View
      style={{
        backgroundColor: colors.primary,
        borderRadius: 1000,
        paddingHorizontal: spacing[2],
        paddingVertical: spacing[0.5],
      }}
    >
      <SurfaceText font="text-xs" weight="bold" color="onPrimary">
        {t('workout.up_next.label')}
      </SurfaceText>
    </View>
  );
}

/** "Last done 3 days ago · Thu", from when a plan workout was last finished. */
function useLastDoneLabel() {
  const { t } = useTranslate();
  const formatDate = useFormatDate();
  return (lastFinishedMs: number | undefined): string => {
    if (lastFinishedMs === undefined) {
      return t('workout.last_done.never.label');
    }
    const day = LocalDate.ofInstant(Instant.ofEpochMilli(lastFinishedMs));
    const days = day.until(LocalDate.now(), ChronoUnit.DAYS);
    if (days <= 0) {
      return t('workout.last_done.today.label');
    }
    if (days === 1) {
      return t('workout.last_done.yesterday.label');
    }
    return t('workout.last_done.days_ago.label', {
      count: days.toString(),
      day: formatDate(day, days < 7 ? { weekday: 'short' } : { day: 'numeric', month: 'short' }),
    });
  };
}

export default function Index() {
  const upcomingSessions = useAppSelector((s) => s.program.upcomingSessions);
  const dispatch = useDispatch();
  const currentBodyweight = upcomingSessions.map((x) => x.at(0)?.bodyweight).unwrapOr(undefined);
  const { start, confirmationDialog } = useStartWorkoutWithConfirmation();

  useFocusEffect(() => {
    dispatch(fetchUpcomingSessions());
  });

  const createFreeformSession = () => {
    start(Session.freeformSession(LocalDate.now(), currentBodyweight));
  };

  return (
    <FullHeightScrollView
      scrollStyle={{ paddingHorizontal: spacing.pageHorizontalMargin }}
      contentContainerStyle={{ flexGrow: 1 }}
    >
      <Stack.Screen
        options={{
          title: 'LiftLog',
          headerBackVisible: false,
        }}
      />
      <PlanMenu onStartEmptyWorkout={createFreeformSession} />
      <Remote
        value={upcomingSessions}
        success={(upcoming) => {
          return <ListUpcomingWorkouts startSession={start} upcoming={upcoming} />;
        }}
      />
      {confirmationDialog}
    </FullHeightScrollView>
  );
}
