import {
  CardioExerciseBlueprint,
  ExerciseBlueprint,
  MovementKey,
  ProgressionKey,
  SessionBlueprint,
  WeightedExerciseBlueprint,
} from '@/models/blueprint-models';
import { WeightUnit } from '@/models/weight';
import {
  RecordedCardioExercise,
  RecordedCardioExerciseSet,
  RecordedExercise,
  RecordedWeightedExercise,
  Session,
  WeightedHistory,
} from '@/models/session-models';
import { readLatestCardioExercises, readSessionToContinueFrom, readWeightedHistory } from '@/db/sessions';
import type { RootState } from '@/store';
import { uuid } from '@/utils/uuid';
import { LocalDate } from '@js-joda/core';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

export class SessionService {
  constructor(
    private db: ExpoSQLiteDatabase,
    private getState: () => RootState,
  ) {}

  async *getUpcomingSessions(sessionBlueprints: SessionBlueprint[]): AsyncIterableIterator<Session> {
    if (!sessionBlueprints.length) {
      return;
    }

    const exercises = sessionBlueprints.flatMap((session) => session.exercises);
    const weightedHistory = readWeightedHistory(this.db, {
      movementKeys: [
        ...new Set(
          exercises.filter((e) => e instanceof WeightedExerciseBlueprint).map((exercise) => exercise.movementKey()),
        ),
      ],
    });
    const latestCardio = readLatestCardioExercises(
      this.db,
      exercises.filter((e) => e instanceof CardioExerciseBlueprint).map((exercise) => exercise.progressionKey()),
    );
    const previousSession = readSessionToContinueFrom(this.db);

    // Track the plan position by index so the plan is walked in order.
    // Matching only by name would stall on duplicate-named workouts, always
    // resolving to the first one and never advancing past it.
    let index = previousSession ? sessionBlueprints.findIndex((x) => x.name === previousSession.name) : -1;

    while (true) {
      index = (index + 1) % sessionBlueprints.length;
      yield this.createNewSession(sessionBlueprints[index]!, weightedHistory, latestCardio).with({
        bodyweight: previousSession?.bodyweight,
      });
    }
  }

  private createNewSession(
    sessionBlueprint: SessionBlueprint,
    weightedHistory: ReadonlyMap<MovementKey, WeightedHistory>,
    latestCardio: Record<ProgressionKey, RecordedCardioExercise | undefined>,
  ): Session {
    const unit = this.getDefaultWeightUnit();
    const getNextExercise = (e: ExerciseBlueprint): RecordedExercise => {
      if (e instanceof CardioExerciseBlueprint) {
        const lastExercise = latestCardio[e.progressionKey()];
        return RecordedCardioExercise.empty(e).with({
          sets: e.sets.map((s, i) =>
            RecordedCardioExerciseSet.empty(s).with({
              incline: lastExercise?.sets[i]?.incline,
              resistance: lastExercise?.sets[i]?.resistance,
            }),
          ),
        });
      }
      return RecordedWeightedExercise.empty(e, unit, weightedHistory.get(e.movementKey())?.sets);
    };
    return new Session(
      uuid(),
      sessionBlueprint,
      sessionBlueprint.exercises.map(getNextExercise),
      LocalDate.now(),
      undefined,
      undefined,
    );
  }

  private getDefaultWeightUnit(): WeightUnit {
    return this.getState().settings.useImperialUnits ? 'pounds' : 'kilograms';
  }
}
