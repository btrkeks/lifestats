import { SessionBlueprint } from '@/models/blueprint-models';
import { WeightUnit } from '@/models/weight';
import { Session, StartingHistory } from '@/models/session-models';
import { createEmptyRecordedExercise } from '@/models/session-models/recorded-exercise';
import { readSessionToContinueFrom, readStartingHistory } from '@/db/sessions';
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

    const history = readStartingHistory(
      this.db,
      sessionBlueprints.flatMap((session) => session.exercises),
    );
    const previousSession = readSessionToContinueFrom(this.db);

    // Track the plan position by index so the plan is walked in order.
    // Matching only by name would stall on duplicate-named workouts, always
    // resolving to the first one and never advancing past it.
    let index = previousSession ? sessionBlueprints.findIndex((x) => x.name === previousSession.name) : -1;

    while (true) {
      index = (index + 1) % sessionBlueprints.length;
      yield this.createNewSession(sessionBlueprints[index]!, history).with({
        bodyweight: previousSession?.bodyweight,
      });
    }
  }

  private createNewSession(sessionBlueprint: SessionBlueprint, history: StartingHistory): Session {
    const unit = this.getDefaultWeightUnit();
    return new Session(
      uuid(),
      sessionBlueprint,
      sessionBlueprint.exercises.map((e) => createEmptyRecordedExercise(e, unit, history)),
      LocalDate.now(),
      undefined,
      undefined,
    );
  }

  private getDefaultWeightUnit(): WeightUnit {
    return this.getState().settings.useImperialUnits ? 'pounds' : 'kilograms';
  }
}
