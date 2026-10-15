import {
  CardioExerciseBlueprint,
  ExerciseBlueprint,
  Rest,
  SessionBlueprint,
  WeightedExerciseBlueprint,
} from '@/models/blueprint-models';
import { TemporalComparer } from '@/models/comparers';
import { SessionJSON, fromLocalDateJSON, toLocalDateJSON } from '@/models/storage/versions/latest';
import { Weight, WeightUnit } from '@/models/weight';
import { indexed } from '@/utils/enumerable';
import { Duration, LocalDate, OffsetDateTime } from '@js-joda/core';
import { match } from 'ts-pattern';
import Enumerable from 'linq';
import { P } from 'ts-pattern';
import { uuid } from '@/utils/uuid';
import { equal } from '@/models/session-models/helpers';
import { RecordedCardioExercise, RecordedCardioExerciseSet } from '@/models/session-models/recorded-cardio-exercise';
import {
  RecordedExercise,
  StartingHistory,
  createEmptyRecordedExercise,
  fromRecordedExerciseJSON,
} from '@/models/session-models/recorded-exercise';
import {
  PotentialSet,
  RecordedWeightedExercise,
  WeightedChange,
} from '@/models/session-models/recorded-weighted-exercise';
import { RestTimer } from '@/models/session-models/rest-timer';
import { IndexOutOfBoundsError } from '@/utils/index-out-of-bounds';

export const freeformSessionName = 'Freeform Workout';

/** The rest between sets, from as soon as you may go to when you should. */
export type RestWindow = Readonly<Pick<Rest, 'minRest' | 'maxRest'>>;

export class Session {
  constructor(
    readonly id: string,
    readonly blueprint: SessionBlueprint,
    readonly recordedExercises: RecordedExercise[],
    readonly date: LocalDate,
    readonly bodyweight: Weight | undefined,
    readonly restTimer: RestTimer | undefined,
  ) {}
  get duration(): Duration | undefined {
    return this.lastExercise?.latestTime && this.firstExercise?.earliestTime
      ? Duration.between(this.firstExercise.earliestTime, this.lastExercise.latestTime)
      : undefined;
  }

  static fromJSON(json: SessionJSON): Session {
    return new Session(
      json.id,
      SessionBlueprint.fromJSON({
        ...json.blueprint,
        version: 6,
        exercises: json.recordedExercises.map((x) => x.blueprint),
      }),
      json.recordedExercises.map(fromRecordedExerciseJSON),
      fromLocalDateJSON(json.date),
      json.bodyweight ? Weight.fromJSON(json.bodyweight) : undefined,
      undefined,
    );
  }

  static getEmptySession(blueprint: SessionBlueprint, defaultWeightUnit: WeightUnit): Session {
    function getNextExercise(e: ExerciseBlueprint) {
      return match(e)
        .with(
          P.instanceOf(WeightedExerciseBlueprint),
          (we) =>
            new RecordedWeightedExercise(
              we,
              we.plannedSets.map((s) => new PotentialSet(undefined, new Weight(0, defaultWeightUnit), s.reps)),
              undefined,
            ),
        )
        .with(P.instanceOf(CardioExerciseBlueprint), (ce) => RecordedCardioExercise.empty(ce))
        .exhaustive();
    }
    return new Session(
      uuid(),
      blueprint,
      blueprint.exercises.map(getNextExercise),
      LocalDate.now(),
      undefined,
      undefined,
    );
  }

  withNoNilWeights(fallbackWeightUnit: WeightUnit): Session {
    return this.with({
      recordedExercises: this.recordedExercises.map((re) =>
        re instanceof RecordedWeightedExercise
          ? re.with({
              potentialSets: re.potentialSets.map((ps) =>
                ps.with({
                  weight: ps.weight.with({
                    unit: ps.weight.unit === 'nil' ? fallbackWeightUnit : ps.weight.unit,
                  }),
                }),
              ),
            })
          : re,
      ),
    });
  }

  equals(other: Session | undefined): boolean {
    if (!other) {
      return false;
    }

    return (
      this.id === other.id &&
      this.date.equals(other.date) &&
      equal(this.bodyweight, other.bodyweight) &&
      this.blueprint.equals(other.blueprint) &&
      this.recordedExercises.length === other.recordedExercises.length &&
      this.recordedExercises.every((exercise, index) => exercise.equals(other.recordedExercises[index]))
    );
  }

  with(other: Partial<Session>) {
    return new Session(
      'id' in other ? (other.id ?? this.id) : this.id,
      'blueprint' in other ? (other.blueprint ?? this.blueprint) : this.blueprint,
      'recordedExercises' in other ? (other.recordedExercises ?? this.recordedExercises) : this.recordedExercises,
      'date' in other ? (other.date ?? this.date) : this.date,
      'bodyweight' in other ? other.bodyweight : this.bodyweight,
      'restTimer' in other ? other.restTimer : this.restTimer,
    );
  }

  withEditedExercise(exerciseIndex: number, newBlueprint: ExerciseBlueprint, useImperialUnits: boolean): Session {
    // oxlint-disable-next-line typescript/no-this-alias
    let session: Session = this;
    const existingExercise = session.recordedExercises[exerciseIndex];
    if (!existingExercise) {
      throw new IndexOutOfBoundsError(exerciseIndex, session.recordedExercises);
    }

    session = session.with({
      blueprint: session.blueprint.with({
        exercises: session.blueprint.exercises.with(exerciseIndex, newBlueprint),
      }),
    });
    if (existingExercise.blueprint.type !== newBlueprint.type) {
      session = session.withExercise(
        exerciseIndex,
        createEmptyRecordedExercise(newBlueprint, useImperialUnits ? 'pounds' : 'kilograms'),
      );
    } else {
      const weightedExistingExercise =
        session.recordedExercises[exerciseIndex]!.type === 'RecordedWeightedExercise'
          ? session.recordedExercises[exerciseIndex]
          : undefined;
      if (weightedExistingExercise) {
        const plannedBefore = weightedExistingExercise.blueprint.plannedSets.length;
        // Sets added during the workout sit past the plan and are not the plan's to resize.
        const addedSets = weightedExistingExercise.potentialSets.slice(plannedBefore);
        session = session.withExercise(
          exerciseIndex,
          weightedExistingExercise.with({
            blueprint: newBlueprint as WeightedExerciseBlueprint,
            potentialSets: (newBlueprint as WeightedExerciseBlueprint).plannedSets
              .map(
                (_, index) =>
                  (index < plannedBefore ? weightedExistingExercise.potentialSets.at(index) : undefined) ??
                  new PotentialSet(undefined, weightedExistingExercise.maxWeight),
              )
              .concat(addedSets),
          }),
        );
      }

      const cardioExistingExercise =
        session.recordedExercises[exerciseIndex]!.type === 'RecordedCardioExercise'
          ? session.recordedExercises[exerciseIndex]
          : undefined;

      if (cardioExistingExercise) {
        session = session.withExercise(
          exerciseIndex,
          cardioExistingExercise.with({
            blueprint: newBlueprint as CardioExerciseBlueprint,
            sets: (newBlueprint as CardioExerciseBlueprint).sets.map((set, i) =>
              RecordedCardioExerciseSet.empty(set).with({
                // Basically allows us to use values from set, even if there are more sets now and it would be undefined
                // oxlint-disable-next-line typescript/no-misused-spread
                ...cardioExistingExercise.sets[i],
                blueprint: set,
              }),
            ),
          }),
        );
      }
    }
    return session;
  }

  withAddedExercise(exercise: ExerciseBlueprint, useImperialUnits: boolean): Session {
    return this.with({
      blueprint: this.blueprint.with({
        exercises: this.blueprint.exercises.concat(exercise),
      }),
      recordedExercises: this.recordedExercises.concat(
        createEmptyRecordedExercise(exercise, useImperialUnits ? 'pounds' : 'kilograms'),
      ),
    });
  }

  withUpdatedDate(date: LocalDate): Session {
    const originalDate = this.date;
    const newDate = date;

    // Gather all unique, non-null completion dates from all sets
    const allCompletionDates = this.recordedExercises
      .flatMap((re) =>
        re.type === 'RecordedWeightedExercise'
          ? re.potentialSets.map((ps) => ps.set?.completionDateTime?.toLocalDate())
          : re.sets.map((s) => s.completionDateTime?.toLocalDate()),
      )
      .filter((d): d is LocalDate => d !== undefined);

    // If all sets have the same completion date, use absolute date
    const useAbsoluteDate =
      allCompletionDates.length > 0 && new Set(allCompletionDates.map((d) => d.toString())).size === 1;

    function getAdjustedDate(setDate: LocalDate): LocalDate {
      if (useAbsoluteDate) {
        return newDate;
      }
      // Maintain relative offset if sets cross midnight
      const dayOffset = setDate.toEpochDay() - originalDate.toEpochDay();
      return newDate.plusDays(dayOffset);
    }

    // Update all sets' completionDateTime
    const newExercises = this.recordedExercises.map((re) => {
      if (re.type === 'RecordedWeightedExercise') {
        return re.withAllSets((ps) => {
          if (ps.set && ps.set.completionDateTime) {
            const setDate = ps.set.completionDateTime.toLocalDate();
            return ps.with({
              set: ps.set.with({
                completionDateTime: ps.set.completionDateTime
                  .toLocalTime()
                  .atDate(getAdjustedDate(setDate))
                  .atOffset(ps.set.completionDateTime.offset()),
              }),
            });
          }
          return ps;
        });
      } else {
        return re.withAllSets((set) => {
          if (set && set.completionDateTime) {
            const setDate = set.completionDateTime.toLocalDate();
            return set.with({
              completionDateTime: set.completionDateTime
                .toLocalTime()
                .atDate(getAdjustedDate(setDate))
                .atOffset(set.completionDateTime.offset()),
            });
          }
          return set;
        });
      }
    });

    return this.with({
      recordedExercises: newExercises,
      date,
    });
  }

  withNothingCompleted(): Session {
    return this.with({
      recordedExercises: this.recordedExercises.map((re) => re.withNothingCompleted()),
    });
  }

  /**
   * Applies a set-table change and the rest policy in one transition, so the screen dispatches once.
   * The rest timer re-anchors at the newest completion whenever the set of checked sets changes, and
   * clears when nothing is checked. Edits to checked sets, weights and added or removed unchecked sets
   * leave it alone.
   */
  withWeightedChange(exerciseIndex: number, change: WeightedChange, time: OffsetDateTime): Session {
    const before = this.recordedExercises[exerciseIndex];
    if (!before) {
      throw new IndexOutOfBoundsError(exerciseIndex, this.recordedExercises);
    }
    if (!(before instanceof RecordedWeightedExercise)) {
      return this;
    }
    const after = before.withChange(change, time);
    if (after === before) {
      return this;
    }
    const updated = this.withExercise(exerciseIndex, after);
    if (checkedIndices(before) === checkedIndices(after)) {
      return updated;
    }
    const latest = updated.lastExercise?.latestTime;
    return updated.with({ restTimer: latest ? new RestTimer(latest) : undefined });
  }

  withExercise(exerciseIndex: number, exercise: RecordedExercise): Session {
    return this.with({
      recordedExercises: this.recordedExercises.with(exerciseIndex, exercise),
      blueprint: this.blueprint.with({
        exercises: this.blueprint.exercises.with(exerciseIndex, exercise.blueprint),
      }),
    });
  }

  /**
   * Puts another movement in the slot as if the workout had opened with it: every set unchecked, as many
   * as before, seeded from `history` for the new name. Rest, superset link and cardio targets belong to
   * the slot and stay; notes and link described the old movement and go. A weighted slot takes the
   * resistance last used for the new movement. The rest timer re-anchors when checked sets disappear.
   */
  withReplacedExercise(exerciseIndex: number, name: string, unit: WeightUnit, history: StartingHistory): Session {
    const before = this.recordedExercises[exerciseIndex];
    if (!before) {
      throw new IndexOutOfBoundsError(exerciseIndex, this.recordedExercises);
    }
    const renamed = before.blueprint.with({ name, notes: '', link: '' });
    const blueprint =
      renamed instanceof WeightedExerciseBlueprint
        ? renamed.with({
            resistance: history.weighted.get(renamed.movementKey())?.sets.values().next().value?.context.resistance,
          })
        : renamed;
    let after = createEmptyRecordedExercise(blueprint, unit, history);
    const addedSets =
      before instanceof RecordedWeightedExercise
        ? before.potentialSets.length - before.blueprint.plannedSets.length
        : 0;
    for (let i = 0; i < addedSets && after instanceof RecordedWeightedExercise; i++) {
      after = after.withSetAdded();
    }
    const replaced = this.withExercise(exerciseIndex, after);
    if (!before.isStarted) {
      return replaced;
    }
    const latest = replaced.lastExercise?.latestTime;
    return replaced.with({ restTimer: latest ? new RestTimer(latest) : undefined });
  }

  withRemovedExercise(exerciseIndex: number): Session {
    return this.with({
      recordedExercises: this.recordedExercises.toSpliced(exerciseIndex, 1),
      blueprint: this.blueprint.with({
        exercises: this.blueprint.exercises.toSpliced(exerciseIndex, 1),
      }),
    });
  }

  toJSON(): SessionJSON {
    return {
      version: 7,
      blueprint: this.blueprint.toJSON(),
      bodyweight: this.bodyweight?.toJSON(),
      date: toLocalDateJSON(this.date),
      id: this.id,
      recordedExercises: this.recordedExercises.map((x) => x.toJSON()),
    };
  }

  static freeformSession(date: LocalDate, bodyweight: Weight | undefined): Session {
    return EmptySession.with({
      id: uuid(),
      date: date,
      bodyweight,
      blueprint: EmptySession.blueprint.with({ name: freeformSessionName }),
    });
  }

  withName(name: string): Session {
    return this.with({ blueprint: this.blueprint.with({ name }) });
  }

  get totalWeightLifted(): Weight {
    return this.recordedExercises.reduce(
      (b, ex) =>
        b.plus(ex instanceof RecordedWeightedExercise ? ex.totalWeightLiftedWith(this.bodyweight) : Weight.NIL),
      Weight.NIL,
    );
  }

  get isComplete() {
    return this.recordedExercises.every((x) => x.isComplete);
  }

  get isStarted(): boolean {
    return this.recordedExercises.some((x) => x.isStarted);
  }

  get runningCardioSet():
    | { exerciseIndex: number; setIndex: number; exercise: RecordedCardioExercise; set: RecordedCardioExerciseSet }
    | undefined {
    for (const [exerciseIndex, exercise] of this.recordedExercises.entries()) {
      if (!(exercise instanceof RecordedCardioExercise)) {
        continue;
      }
      const setIndex = exercise.sets.findIndex((set) => set.isTimerRunning);
      const set = exercise.sets[setIndex];
      if (set) {
        return { exerciseIndex, setIndex, exercise, set };
      }
    }
    return undefined;
  }

  cardioSetAt(exerciseIndex: number, setIndex: number): RecordedCardioExerciseSet | undefined {
    const exercise = this.recordedExercises[exerciseIndex];
    return exercise instanceof RecordedCardioExercise ? exercise.sets[setIndex] : undefined;
  }

  withCardioSet(
    exerciseIndex: number,
    setIndex: number,
    update: (set: RecordedCardioExerciseSet) => RecordedCardioExerciseSet,
    now: OffsetDateTime,
  ): Session {
    const exercise = this.recordedExercises[exerciseIndex];
    if (!(exercise instanceof RecordedCardioExercise)) {
      return this;
    }
    return this.withExercise(
      exerciseIndex,
      exercise.withSet(setIndex, (set) => update(set).withCompletionTimeIfCompleted(now)),
    );
  }

  /** Only one cardio clock runs at a time, so whatever another set had going is banked first. */
  withCardioTimerStarted(exerciseIndex: number, setIndex: number, now: OffsetDateTime): Session {
    const running = this.runningCardioSet;
    const banked = running
      ? this.withExercise(
          running.exerciseIndex,
          running.exercise.withAllSets((set) =>
            set.isTimerRunning ? set.withTimerStopped(now).withCompletionTimeIfCompleted(now) : set,
          ),
        )
      : this;

    const exercise = banked.recordedExercises[exerciseIndex];
    if (!(exercise instanceof RecordedCardioExercise)) {
      return banked;
    }
    return banked.withExercise(
      exerciseIndex,
      exercise.withSet(setIndex, (set) => set.withTimerStarted(now)),
    );
  }

  get nextExercise(): RecordedExercise | undefined {
    const recordedExercises = this.recordedExercises;
    const cardioExerciseWithRunningTimer = this.runningCardioSet?.exercise;
    if (cardioExerciseWithRunningTimer) {
      return cardioExerciseWithRunningTimer;
    }
    const latestExerciseIndex = Enumerable.from(recordedExercises)
      .select(indexed)
      .where((x) => x.item.isStarted)
      .orderByDescending(({ item }) => item.latestTime, TemporalComparer)
      .select((x) => x.index)
      .firstOrDefault(-1);

    const latestExerciseSupersetsWithNext = match(latestExerciseIndex)
      .with(-1, () => false)
      .with(recordedExercises.length - 1, () => false) // can never superset with next if its the last exercise
      .otherwise(
        (i) =>
          recordedExercises[i] instanceof RecordedWeightedExercise && recordedExercises[i].blueprint.supersetWithNext,
      );

    const latestExerciseSupersetsWithPrevious = match(latestExerciseIndex)
      .with(P.union(-1, 0), () => false)
      .otherwise((i) => {
        const prevIndex = i - 1;
        return (
          recordedExercises[prevIndex] instanceof RecordedWeightedExercise &&
          recordedExercises[prevIndex].blueprint.supersetWithNext
        );
      });

    if (latestExerciseSupersetsWithNext && !recordedExercises[latestExerciseIndex + 1]?.isComplete) {
      return recordedExercises[latestExerciseIndex + 1];
    }

    // loop back to the original exercise in the case of a superset chain
    if (latestExerciseSupersetsWithPrevious) {
      let indexToJumpBackTo = latestExerciseIndex - 1;
      while (
        indexToJumpBackTo >= 0 &&
        recordedExercises[indexToJumpBackTo] instanceof RecordedWeightedExercise &&
        (recordedExercises[indexToJumpBackTo]!.blueprint as WeightedExerciseBlueprint).supersetWithNext
      ) {
        indexToJumpBackTo--;
      }
      // We are now at an exercise which is not supersetting with the next,
      // so jump forward to the next exercise
      indexToJumpBackTo++;
      // Now jump to the first exercise which has remaining sets in the chain
      while (indexToJumpBackTo < recordedExercises.length && recordedExercises[indexToJumpBackTo]!.isComplete) {
        indexToJumpBackTo++;
      }

      if (indexToJumpBackTo < recordedExercises.length) {
        return recordedExercises[indexToJumpBackTo];
      }
    }

    let result: RecordedExercise | undefined = undefined;
    let maxEpochSecond = Number.MIN_VALUE;

    for (const recordedExercise of recordedExercises) {
      if (!recordedExercise.isComplete) {
        const latestTime = recordedExercise.latestTime;
        const epochSecond = latestTime?.toEpochSecond() ?? Number.MIN_VALUE;

        if (epochSecond > maxEpochSecond || !result) {
          maxEpochSecond = epochSecond;
          result = recordedExercise;
        }
      }
    }
    return result;
  }

  /** What the last completed set earns: a weighted exercise rests per exercise, cardio per set, if at all. */
  get restWindow(): RestWindow | undefined {
    const exercise = this.lastExercise;
    if (exercise instanceof RecordedWeightedExercise) {
      return exercise.blueprint.restBetweenSets;
    }
    return exercise?.lastCompletedSet?.blueprint.restBetweenSets;
  }

  /** When the next weighted set is due; cardio schedules no next-set notification. */
  get restTimerEndTime(): OffsetDateTime | undefined {
    if (!this.restTimer || this.restTimer.isPaused || !this.nextExercise) {
      return undefined;
    }
    const rest = this.lastExercise instanceof RecordedWeightedExercise ? this.restWindow?.minRest : undefined;
    return rest && !rest.equals(Duration.ZERO) ? this.restTimer.startedAt.plus(rest) : undefined;
  }

  get lastExercise(): RecordedExercise | undefined {
    return Enumerable.from(this.recordedExercises)
      .where((x) => x.isStarted)
      .defaultIfEmpty(undefined)
      .maxBy((x) => x.latestTime?.toInstant().toEpochMilli());
  }

  get latestWeightedExercise(): RecordedWeightedExercise | undefined {
    return Enumerable.from(this.recordedExercises)
      .where((x) => x.isStarted)
      .where((x) => x instanceof RecordedWeightedExercise)
      .defaultIfEmpty(undefined)
      .maxBy((x) => x.latestTime?.toInstant().toEpochMilli());
  }

  get firstExercise(): RecordedExercise | undefined {
    return Enumerable.from(this.recordedExercises)
      .where((x) => x.isStarted)
      .defaultIfEmpty(undefined)
      .minBy((x) => x.latestTime?.toInstant().toEpochMilli());
  }

  get isFreeform(): boolean {
    return this.blueprint.name === freeformSessionName;
  }
}

function checkedIndices(exercise: RecordedWeightedExercise): string {
  return exercise.potentialSets.flatMap((s, i) => (s.set ? [i] : [])).join(',');
}

export const EmptySession: Session = new Session(
  '00000000-0000-0000-0000-000000000000',
  new SessionBlueprint('', [], ''),
  [],
  LocalDate.MIN,
  undefined,
  undefined,
);
