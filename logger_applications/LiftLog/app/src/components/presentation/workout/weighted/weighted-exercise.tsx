import Button from '@/components/presentation/foundation/button';
import ExerciseSection from '@/components/presentation/workout/exercise-section';
import { SetTableHeader, SetTableRow } from '@/components/presentation/workout/weighted/set-row';
import { columnWeightUnit, setRows, WeightColumn } from '@/components/presentation/workout/weighted/set-rows';
import { spacing } from '@/hooks/useAppTheme';
import { usePreferredWeightUnit } from '@/hooks/usePreferredWeightUnit';
import { RecordedWeightedExercise, WeightedChange } from '@/models/session-models';
import { Updater } from '@/utils/types';
import { T } from '@tolgee/react';
import { View } from 'react-native';

interface WeightedExerciseProps {
  recordedExercise: RecordedWeightedExercise;
  previousLineageExercise: RecordedWeightedExercise | undefined;
  toStartNext: boolean;
  isReadonly: boolean;
  showPreviousButton: boolean;

  /** Every set-table edit; the owner applies it with the time and the rest policy. */
  onChange: (change: WeightedChange) => void;
  updateExercise: (update: Updater<RecordedWeightedExercise>) => void;
  onEditExercise: (() => void) | undefined;
  onRemoveExercise: () => void;
}

export default function WeightedExercise(props: WeightedExerciseProps) {
  const { recordedExercise, onChange } = props;
  const preferredUnit = usePreferredWeightUnit();
  const column: WeightColumn = {
    unit: columnWeightUnit(recordedExercise, preferredUnit),
    resistance: recordedExercise.blueprint.resistance,
  };
  const nextSetIndex = props.toStartNext && !props.isReadonly ? recordedExercise.currentSetIndex : -1;

  return (
    <ExerciseSection
      recordedExercise={recordedExercise}
      previousLineageExercise={props.previousLineageExercise}
      toStartNext={props.toStartNext}
      isReadonly={props.isReadonly}
      showPreviousButton={props.showPreviousButton}
      updateExercise={props.updateExercise}
      onEditExercise={props.onEditExercise}
      onRemoveExercise={props.onRemoveExercise}
    >
      <View style={{ gap: spacing[1] }}>
        <SetTableHeader column={column} />
        {setRows(recordedExercise, props.previousLineageExercise).map((row) => (
          <SetTableRow
            key={row.index}
            row={row}
            column={column}
            isReadonly={props.isReadonly}
            isNext={row.index === nextSetIndex}
            onChange={onChange}
          />
        ))}
        {!props.isReadonly && (
          <View style={{ flexDirection: 'row', justifyContent: 'center', gap: spacing[2] }}>
            {recordedExercise.canRemoveLastSet && (
              <Button testID="remove-set" icon="remove" onPress={() => onChange({ kind: 'removeLastSet' })}>
                <T keyName="exercise.remove_set.button" />
              </Button>
            )}
            <Button testID="add-set" icon="add" onPress={() => onChange({ kind: 'addSet' })}>
              <T keyName="exercise.add_set.button" />
            </Button>
          </View>
        )}
      </View>
    </ExerciseSection>
  );
}
