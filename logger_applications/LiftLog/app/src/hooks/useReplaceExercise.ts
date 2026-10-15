import { useServices } from '@/components/smart/services-provider';
import { readStartingHistory } from '@/db/sessions';
import { useExerciseSearch } from '@/hooks/useExerciseSearch';
import { usePreferredWeightUnit } from '@/hooks/usePreferredWeightUnit';
import { Session } from '@/models/session-models';
import { Updater } from '@/utils/types';
import { useRef } from 'react';

export function useReplaceExercise(session: Session, updateSession: (update: Updater<Session>) => void) {
  const { db } = useServices();
  const unit = usePreferredWeightUnit();
  const exerciseIndex = useRef<number>(undefined);
  const openSearch = useExerciseSearch((exercise) => {
    const index = exerciseIndex.current;
    if (index === undefined) {
      return;
    }
    const slot = session.recordedExercises[index];
    if (!slot) {
      return;
    }
    const history = readStartingHistory(db, [slot.blueprint.with({ name: exercise.name })]);
    updateSession((s) => s.withReplacedExercise(index, exercise.name, unit, history));
  });

  return (index: number) => {
    exerciseIndex.current = index;
    openSearch('');
  };
}
