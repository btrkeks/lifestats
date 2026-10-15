import Button from '@/components/presentation/foundation/button';
import ExerciseSection from '@/components/presentation/workout/exercise-section';
import { CellInputHandle } from '@/components/presentation/foundation/editors/cell-input';
import { SetTableHeader, SetTableRow } from '@/components/presentation/workout/weighted/set-row';
import {
  CellAddress,
  columnWeightUnit,
  setRows,
  WeightColumn,
} from '@/components/presentation/workout/weighted/set-rows';
import { SurfaceText } from '@/components/presentation/foundation/surface-text';
import { rounding, spacing, useAppTheme } from '@/hooks/useAppTheme';
import { usePreferredWeightUnit } from '@/hooks/usePreferredWeightUnit';
import { loadContextOf, RecordedWeightedExercise, WeightedChange, WeightedHistory } from '@/models/session-models';
import { isStalled, SessionPeak } from '@/models/set-progress';
import { Weight } from '@/models/weight';
import { Updater } from '@/utils/types';
import { T, useTranslate } from '@tolgee/react';
import { useRef } from 'react';
import { View } from 'react-native';

interface WeightedExerciseProps {
  recordedExercise: RecordedWeightedExercise;
  previous: WeightedHistory | undefined;
  toStartNext: boolean;
  isReadonly: boolean;
  showPreviousButton: boolean;
  /**
   * Only the live workout passes this: it turns on rep targets, better-set marks and the stall flag.
   * `recent` is this movement's latest session peaks, newest first.
   */
  progress?: { bodyweight: Weight | undefined; recent: readonly SessionPeak[] } | undefined;

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
  const cells = useRef({ weight: new Map<number, CellInputHandle>(), reps: new Map<number, CellInputHandle>() });
  const cellRef =
    ({ setIndex, field }: CellAddress) =>
    (handle: CellInputHandle | null) => {
      if (!handle) {
        return;
      }
      cells.current[field].set(setIndex, handle);
      return () => {
        cells.current[field].delete(setIndex);
      };
    };
  const focusCell = ({ setIndex, field }: CellAddress) => cells.current[field].get(setIndex)?.focus();
  const nextSetIndex = props.toStartNext && !props.isReadonly ? recordedExercise.currentSetIndex : -1;
  const live = props.progress && {
    context: loadContextOf(recordedExercise.blueprint.resistance, props.progress.bodyweight),
    recent: props.progress.recent,
  };
  const stalled =
    !!live &&
    isStalled({
      recent: live.recent,
      today: {
        context: live.context,
        sets: recordedExercise.potentialSets.flatMap((ps) =>
          ps.set ? [{ weight: ps.weight, reps: ps.set.repsCompleted }] : [],
        ),
      },
    });

  return (
    <ExerciseSection
      recordedExercise={recordedExercise}
      previousNotes={props.previous?.lastNotes}
      toStartNext={props.toStartNext}
      isReadonly={props.isReadonly}
      showPreviousButton={props.showPreviousButton}
      belowTitle={stalled && <StallChip />}
      updateExercise={props.updateExercise}
      onEditExercise={props.onEditExercise}
      onRemoveExercise={props.onRemoveExercise}
    >
      <View style={{ gap: spacing[1] }}>
        <SetTableHeader column={column} />
        {setRows(recordedExercise, props.previous?.sets).map((row, _, rows) => (
          <SetTableRow
            key={row.index}
            row={row}
            column={column}
            isReadonly={props.isReadonly}
            isNext={row.index === nextSetIndex}
            isLast={row.index === rows.length - 1}
            context={live?.context}
            onChange={onChange}
            cellRef={cellRef}
            focusCell={focusCell}
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

function StallChip() {
  const { t } = useTranslate();
  const { colors } = useAppTheme();
  return (
    <View
      testID="exercise-stalled"
      style={{
        alignSelf: 'flex-start',
        marginBottom: spacing[2],
        paddingVertical: spacing[1],
        paddingHorizontal: spacing[2],
        borderRadius: rounding.roundedRectangleRadius,
        backgroundColor: colors.secondaryContainer,
      }}
    >
      <SurfaceText font="text-xs" color="onSecondaryContainer">
        {t('exercise.stalled.label')}
      </SurfaceText>
    </View>
  );
}
