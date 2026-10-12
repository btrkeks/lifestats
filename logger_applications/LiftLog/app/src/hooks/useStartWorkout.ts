import { Session } from '@/models/session-models';
import { useAppSelector } from '@/store';
import {
  deleteStoredSession,
  putStoredSession,
  selectActiveSession,
  setActiveSessionId,
} from '@/store/stored-sessions';
import { useDispatch } from 'react-redux';
import { requestPermissionsAsync } from 'expo-notifications';
import { Platform } from 'react-native';

/**
 * Makes a session the workout in progress. Replacing an unfinished one discards it, which is what the
 * replace prompt promises - an abandoned workout should not turn up in history.
 */
export function useStartWorkout() {
  const dispatch = useDispatch();
  const activeSession = useAppSelector(selectActiveSession);

  return (session: Session) => {
    if (activeSession && activeSession.id !== session.id) {
      dispatch(deleteStoredSession(activeSession.id));
    }
    dispatch(putStoredSession(session));
    dispatch(setActiveSessionId(session.id));
    // The workout notification and rest alerts are posted natively, so nothing else asks Android
    // for permission. Once granted or permanently denied this resolves without a prompt.
    if (Platform.OS === 'android') {
      void requestPermissionsAsync();
    }
  };
}
