import { ExerciseDescriptor } from '@/models/exercise-models';
import { useAppSelector } from '@/store';
import { clearExerciseSearchResult } from '@/store/app';
import { uuid } from '@/utils/uuid';
import { useRouter } from 'expo-router';
import { useEffect, useState } from 'react';
import { Keyboard } from 'react-native';
import { useDispatch } from 'react-redux';

/** Returns a function that opens the exercise search; the exercise picked there is passed to `onSelect`. */
export function useExerciseSearch(onSelect: (exercise: ExerciseDescriptor) => void) {
  const { push } = useRouter();
  const dispatch = useDispatch();
  const [requestId] = useState(() => uuid());
  const searchResult = useAppSelector((x) => x.app.exerciseSearchResult);

  useEffect(() => {
    if (searchResult?.requestId !== requestId) {
      return;
    }
    onSelect(searchResult.exercise);
    dispatch(clearExerciseSearchResult());
  }, [onSelect, searchResult, requestId, dispatch]);

  return (exerciseName: string) => {
    Keyboard.dismiss();
    push({ pathname: '/exercise-search', params: { requestId, exerciseName } });
  };
}
