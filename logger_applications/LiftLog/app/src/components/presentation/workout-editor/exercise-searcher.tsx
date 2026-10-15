import Button from '@/components/presentation/foundation/button';
import { useExerciseSearch } from '@/hooks/useExerciseSearch';
import { ExerciseBlueprint } from '@/models/blueprint-models';
import { ExerciseDescriptor } from '@/models/exercise-models';

interface ExerciseSearcherProps {
  currentExercise: ExerciseBlueprint;
  onSelectExercise: (e: ExerciseDescriptor) => void;
}

export function ExerciseSearcher({ currentExercise, onSelectExercise }: ExerciseSearcherProps) {
  const openSearch = useExerciseSearch(onSelectExercise);

  return (
    <Button icon={'contentPasteSearch'} mode="contained" onPress={() => openSearch(currentExercise.name)}>
      {currentExercise.name}
    </Button>
  );
}
