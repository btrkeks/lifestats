import {
  CardioExerciseBlueprint,
  ExerciseBlueprint,
  MovementKey,
  ProgressionKey,
  WeightedExerciseBlueprint,
} from '@/models/blueprint-models';
import { RecordedCardioExercise } from '@/models/session-models/recorded-cardio-exercise';
import { RecordedWeightedExercise, WeightedHistory } from '@/models/session-models/recorded-weighted-exercise';
import { RecordedExerciseJSON } from '@/models/storage/versions/latest';
import { WeightUnit } from '@/models/weight';
import { match, P } from 'ts-pattern';

export type RecordedExercise = RecordedCardioExercise | RecordedWeightedExercise;

export function fromRecordedExerciseJSON(json: RecordedExerciseJSON): RecordedExercise {
  return match(json)
    .with({ type: 'RecordedCardioExercise' }, RecordedCardioExercise.fromJSON)
    .with({ type: 'RecordedWeightedExercise' }, RecordedWeightedExercise.fromJSON)
    .exhaustive();
}

/** The last performance of each movement a workout can start from, keyed as the blueprints key them. */
export interface StartingHistory {
  readonly weighted: ReadonlyMap<MovementKey, WeightedHistory>;
  readonly cardio: Readonly<Record<ProgressionKey, RecordedCardioExercise | undefined>>;
}

/** Nothing checked, seeded from `history` the way a workout opens each exercise. */
export function createEmptyRecordedExercise(
  blueprint: ExerciseBlueprint,
  unit: WeightUnit,
  history?: StartingHistory,
): RecordedExercise {
  return match(blueprint)
    .with(P.instanceOf(WeightedExerciseBlueprint), (b) =>
      RecordedWeightedExercise.empty(b, unit, history?.weighted.get(b.movementKey())?.sets),
    )
    .with(P.instanceOf(CardioExerciseBlueprint), (b) =>
      RecordedCardioExercise.empty(b, history?.cardio[b.progressionKey()]),
    )
    .exhaustive();
}
