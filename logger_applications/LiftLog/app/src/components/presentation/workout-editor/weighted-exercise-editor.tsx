import FixedIncrementer from '@/components/presentation/foundation/editors/fixed-incrementer';
import { FormRow } from '@/components/presentation/foundation/form-row';
import RestFormat from '@/components/presentation/foundation/rest-format';
import { SegmentedList, SegmentListFormElement } from '@/components/presentation/foundation/segmented-list';
import { SegmentedListSelect } from '@/components/presentation/foundation/segmented-list-select';
import { SegmentedListSwitch } from '@/components/presentation/foundation/segmented-list-switch';
import { RestEditorDialog } from '@/components/presentation/workout-editor/rest-editor-dialog';
import { SharedFieldsEditor } from '@/components/presentation/workout-editor/shared-fields-editor';
import { spacing, useAppTheme } from '@/hooks/useAppTheme';
import { ExerciseBlueprint, Resistance, WeightedExerciseBlueprint } from '@/models/blueprint-models';
import { useAppSelector } from '@/store';
import { TranslationKey, useTranslate } from '@tolgee/react';
import { useState } from 'react';
import { View } from 'react-native';

/**
 * What each choice actually changes, since "resistance" names the field without saying what picking one
 * does to the set counter or to the stats built off it.
 */
const resistanceOptions = [
  {
    value: 'external',
    label: 'exercise.resistance.external.label',
    body: 'exercise.resistance.external.body',
  },
  {
    value: 'bodyweight',
    label: 'exercise.resistance.bodyweight.label',
    body: 'exercise.resistance.bodyweight.body',
  },
  { value: 'none', label: 'exercise.resistance.none.label', body: 'exercise.resistance.none.body' },
] as const satisfies { value: Resistance; label: TranslationKey; body: TranslationKey }[];

export function WeightedExerciseEditor({
  exercise,
  updateExercise,
}: {
  exercise: WeightedExerciseBlueprint;
  updateExercise: (ex: Partial<ExerciseBlueprint>) => void;
}) {
  const { t } = useTranslate();
  const { colors } = useAppTheme();
  const restTimersEnabled = useAppSelector((x) => x.settings.restTimersEnabled);
  const [restDialogOpen, setRestDialogOpen] = useState(false);

  return (
    <View style={{ gap: spacing[2] }}>
      <View style={{ marginBlockEnd: spacing[2] }}>
        <FixedIncrementer
          label={t('exercise.sets.label')}
          onValueChange={(value) => updateExercise(exercise.withSets(value))}
          value={exercise.plannedSets.length}
          testID="exercise-sets"
        />
      </View>

      <SharedFieldsEditor exercise={exercise} updateExercise={updateExercise} />

      {restTimersEnabled && (
        <RestEditorDialog
          onRestUpdated={(restBetweenSets) => updateExercise({ restBetweenSets })}
          rest={exercise.restBetweenSets}
          dialogOpen={restDialogOpen}
          setDialogOpen={setRestDialogOpen}
        />
      )}
      <FormRow>
        <SegmentedList
          items={[
            ...(restTimersEnabled
              ? [
                  <SegmentListFormElement
                    key={1}
                    label={t('rest.rest.label')}
                    icon={'airlineSeatReclineExtraFill'}
                    onPress={() => setRestDialogOpen(true)}
                    right={<RestFormat style={{ color: colors.onSurface }} rest={exercise.restBetweenSets} />}
                  />,
                ]
              : []),
            <SegmentedListSwitch
              key={2}
              label={t('workout.superset_next_exercise.button')}
              icon={'link'}
              value={exercise.supersetWithNext}
              testID="exercise-superset"
              onValueChange={(supersetWithNext) => updateExercise({ supersetWithNext })}
            />,
            <SegmentedListSelect
              key={4}
              label={t('exercise.resistance.label')}
              icon={'directionsRun'}
              testID="load-basis"
              value={exercise.resistance}
              options={resistanceOptions.map(({ value, label }) => ({ value, label: t(label) }))}
              onChange={(resistance: Resistance) => updateExercise({ resistance })}
              supportingText={t(resistanceOptions.find((option) => option.value === exercise.resistance)!.body)}
            />,
          ]}
        />
      </FormRow>
    </View>
  );
}
