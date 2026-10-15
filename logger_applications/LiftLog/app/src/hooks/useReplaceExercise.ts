import { useServices } from '@/components/smart/services-provider';
import { readStartingHistory } from '@/db/sessions';
import { useExerciseSearch } from '@/hooks/useExerciseSearch';
import { usePreferredWeightUnit } from '@/hooks/usePreferredWeightUnit';
import { Session } from '@/models/session-models';
import { Updater } from '@/utils/types';
import { useState } from 'react';

/** Returns a function that opens the exercise search and puts the pick in the slot at `index`. */
export function useReplaceExercise(session: Session, updateSession: (update: Updater<Session>) => void) {
  const { db } = useServices();
  const unit = usePreferredWeightUnit();
  const [exerciseIndex, setExerciseIndex] = useState<number>();
  const openSearch = useExerciseSearch((exercise) => {
    const slot = exerciseIndex === undefined ? undefined : session.recordedExercises[exerciseIndex];
    if (!slot || exerciseIndex === undefined) {
      return;
    }
    // Read here rather than inside the update, which runs in a reducer.
    const history = readStartingHistory(db, [slot.blueprint.with({ name: exercise.name })]);
    updateSession((s) => s.withReplacedExercise(exerciseIndex, exercise.name, unit, history));
  });

  return (index: number) => {
    setExerciseIndex(index);
    openSearch('');
  };
}
