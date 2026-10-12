import ConfirmationDialog from '@/components/presentation/foundation/confirmation-dialog';
import SessionComponent from '@/components/smart/session-component';
import SessionMoreMenuComponent from '@/components/smart/session-more-menu-component';
import { useAppSelector } from '@/store';
import { useDispatch } from 'react-redux';
import { selectActiveSession, updateStoredSession } from '@/store/stored-sessions';
import { useFinishWorkout } from '@/hooks/useFinishWorkout';
import { useTranslate } from '@tolgee/react';
import { Stack, useIsFocused, useRouter } from 'expo-router';
import { useKeepAwake } from 'expo-keep-awake';
import { useEffect, useRef, useState } from 'react';

export default function Index() {
  const session = useAppSelector(selectActiveSession);
  const dispatch = useDispatch();
  const finishWorkout = useFinishWorkout(session?.id);
  const showPostWorkoutSummary = useAppSelector((x) => x.settings.showPostWorkoutSummary);
  const keepAwake = useAppSelector((x) => x.settings.keepScreenAwakeDuringWorkout);
  const { dismissTo, push } = useRouter();
  const { t } = useTranslate();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const isFocused = useIsFocused();
  const finishingHere = useRef(false);

  // The workout can also end from the notification, or when it is finished automatically as forgotten.
  useEffect(() => {
    if (!session && isFocused && !finishingHere.current) {
      dismissTo('/');
    }
  }, [session, isFocused, dismissTo]);

  const save = (force = false) => {
    if (!session) {
      return;
    }
    if (!force && !session.isComplete) {
      setConfirmOpen(true);
      return;
    }
    setConfirmOpen(false);
    if (showPostWorkoutSummary) {
      push(`/session/post-workout?sessionId=${encodeURIComponent(session.id)}&source=finished`);
      return;
    }
    finishingHere.current = true;
    const hasDiff = finishWorkout();
    dismissTo('/');
    if (hasDiff) {
      push('/diff-save');
    }
  };
  const showBodyweight = useAppSelector((x) => x.settings.showBodyweight);

  // Finishing clears the active session while this screen is still mounted for the dismiss animation.
  if (!session) {
    return null;
  }

  return (
    <>
      {keepAwake && <KeepAwake />}
      <Stack.Screen
        options={{
          title: session.blueprint.name,
          headerTitleAlign: 'left',
        }}
      />
      <SessionMoreMenuComponent session={session} isActiveWorkout save={save} />
      <SessionComponent
        session={session}
        updateSession={(update) => dispatch(updateStoredSession({ sessionId: session.id, update }))}
        isActiveWorkout
        showBodyweight={showBodyweight}
        openPostWorkoutSummary={() =>
          push(`/session/post-workout?sessionId=${encodeURIComponent(session.id)}&source=live`)
        }
      />
      <ConfirmationDialog
        okText={t('generic.finish.button')}
        onOk={() => save(true)}
        onCancel={() => setConfirmOpen(false)}
        textContent={t('workout.finish.incomplete.body')}
        headline={t('workout.finish.confirm.title')}
        open={confirmOpen}
      />
    </>
  );
}

/**
 * Allows us to conditionally keep the screen awake, as we cannot use hooks conditionally
 */
function KeepAwake() {
  useKeepAwake();
  return <></>;
}
