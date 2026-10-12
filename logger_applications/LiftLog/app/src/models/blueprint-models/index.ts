import { Duration, LocalDate } from '@js-joda/core';
import BigNumber from 'bignumber.js';
import { match } from 'ts-pattern';
import {
  CardioExerciseBlueprintJSON,
  CardioExerciseSetBlueprintJSON,
  CardioTargetJSON,
  DistanceJSON,
  ExerciseBlueprintJSON,
  ProgramBlueprintJSON,
  ProgressionRuleJSON,
  RestJSON,
  SessionBlueprintJSON,
  WeightedExerciseBlueprintJSON,
  fromBigNumberJSON,
  fromDurationJSON,
  fromLocalDateJSON,
  toBigNumberJSON,
  toDurationJSON,
  toLocalDateJSON,
} from '../storage/versions/latest';

export class ProgramBlueprint {
  constructor(
    readonly name: string,
    readonly sessions: SessionBlueprint[],
    readonly lastEdited: LocalDate,
  ) {}

  static fromJSON(json: ProgramBlueprintJSON): ProgramBlueprint {
    return new ProgramBlueprint(
      json.name,
      json.sessions.map(SessionBlueprint.fromJSON),
      fromLocalDateJSON(json.lastEdited),
    );
  }

  equals(other: ProgramBlueprint | undefined) {
    if (!other) {
      return false;
    }
    if (other === this) {
      return true;
    }

    return (
      this.name === other.name &&
      this.lastEdited.equals(other.lastEdited) &&
      this.sessions.length === other.sessions.length &&
      this.sessions.every((session, index) => session.equals(other.sessions[index]))
    );
  }

  toJSON(): ProgramBlueprintJSON {
    return {
      version: 3,
      name: this.name,
      sessions: this.sessions.map((session) => session.toJSON()),
      lastEdited: toLocalDateJSON(this.lastEdited),
    };
  }

  with(other: Partial<ProgramBlueprint>): ProgramBlueprint {
    return new ProgramBlueprint(
      other.name ?? this.name,
      other.sessions ?? this.sessions,
      other.lastEdited ?? this.lastEdited,
    );
  }

  withName(name: string): ProgramBlueprint {
    return this.with({ name });
  }

  withSessions(sessions: SessionBlueprint[]): ProgramBlueprint {
    return this.with({ sessions });
  }

  withSession(sessionIndex: number, update: (session: SessionBlueprint) => SessionBlueprint): ProgramBlueprint {
    const session = this.sessions[sessionIndex];
    if (!session) {
      return this;
    }
    return this.with({ sessions: this.sessions.with(sessionIndex, update(session)) });
  }

  withAddedSession(session: SessionBlueprint): ProgramBlueprint {
    return this.with({ sessions: [...this.sessions, session] });
  }

  withoutSession(session: SessionBlueprint): ProgramBlueprint {
    return this.with({ sessions: this.sessions.filter((x) => !session.equals(x)) });
  }

  withSessionMovedUp(session: SessionBlueprint): ProgramBlueprint {
    const index = this.sessions.findIndex((x) => session.equals(x));
    return index > 0 ? this.withSessionsSwapped(index - 1, index) : this;
  }

  withSessionMovedDown(session: SessionBlueprint): ProgramBlueprint {
    const index = this.sessions.findIndex((x) => session.equals(x));
    return index >= 0 && index < this.sessions.length - 1 ? this.withSessionsSwapped(index, index + 1) : this;
  }

  private withSessionsSwapped(first: number, second: number): ProgramBlueprint {
    const sessions = [...this.sessions];
    [sessions[first], sessions[second]] = [sessions[second]!, sessions[first]!];
    return this.with({ sessions });
  }
}

export class SessionBlueprint {
  constructor(
    readonly name: string,
    readonly exercises: ExerciseBlueprint[],
    readonly notes: string,
  ) {}

  static fromJSON(json: SessionBlueprintJSON): SessionBlueprint {
    return new SessionBlueprint(json.name, json.exercises.map(fromExerciseBlueprintJSON), json.notes);
  }

  equals(other: SessionBlueprint | undefined) {
    if (!other) {
      return false;
    }
    if (other === this) {
      return true;
    }

    return (
      this.name === other.name &&
      this.notes === other.notes &&
      this.exercises.length === other.exercises.length &&
      this.exercises.every((exercise, index) => exercise.equals(other.exercises[index]))
    );
  }

  toJSON(): SessionBlueprintJSON {
    return {
      version: 6,
      name: this.name,
      exercises: this.exercises.map((exercise) => exercise.toJSON()),
      notes: this.notes,
    };
  }

  with(other: Partial<SessionBlueprint>): SessionBlueprint {
    return new SessionBlueprint(other.name ?? this.name, other.exercises ?? this.exercises, other.notes ?? this.notes);
  }

  withName(name: string): SessionBlueprint {
    return this.with({ name });
  }

  withNotes(notes: string): SessionBlueprint {
    return this.with({ notes });
  }

  withAddedExercise(exercise: ExerciseBlueprint): SessionBlueprint {
    return this.with({ exercises: [...this.exercises, exercise] });
  }

  withoutExercise(exercise: ExerciseBlueprint): SessionBlueprint {
    return this.with({ exercises: this.exercises.filter((x) => !exercise.equals(x)) });
  }

  withExercise(exerciseIndex: number, exercise: ExerciseBlueprint): SessionBlueprint {
    if (exerciseIndex < 0 || exerciseIndex >= this.exercises.length) {
      return this;
    }
    return this.with({ exercises: this.exercises.with(exerciseIndex, exercise) });
  }

  withExerciseMovedUp(exercise: ExerciseBlueprint): SessionBlueprint {
    const index = this.exercises.findIndex((x) => exercise.equals(x));
    return index > 0 ? this.withExercisesSwapped(index - 1, index) : this;
  }

  withExerciseMovedDown(exercise: ExerciseBlueprint): SessionBlueprint {
    const index = this.exercises.findIndex((x) => exercise.equals(x));
    return index >= 0 && index < this.exercises.length - 1 ? this.withExercisesSwapped(index, index + 1) : this;
  }

  private withExercisesSwapped(first: number, second: number): SessionBlueprint {
    const exercises = [...this.exercises];
    [exercises[first], exercises[second]] = [exercises[second]!, exercises[first]!];
    return this.with({ exercises });
  }
}

export type ExerciseBlueprint = WeightedExerciseBlueprint | CardioExerciseBlueprint;

function fromExerciseBlueprintJSON(json: ExerciseBlueprintJSON): ExerciseBlueprint {
  return match(json)
    .with({ type: 'CardioExerciseBlueprint' }, CardioExerciseBlueprint.fromJSON)
    .with({ type: 'WeightedExerciseBlueprint' }, WeightedExerciseBlueprint.fromJSON)
    .exhaustive();
}

export const DistanceUnits = ['metre', 'yard', 'mile', 'kilometre'] as const;
export type DistanceUnit = (typeof DistanceUnits)[number];

export type TimeCardioTarget = {
  type: 'time';
  value: Duration;
};

export type DistanceCardioTarget = {
  type: 'distance';
  value: Distance;
};

export type Distance = {
  value: BigNumber;
  unit: DistanceUnit;
};

export type CardioTarget = TimeCardioTarget | DistanceCardioTarget;

export function matchCardioTarget<T>(
  value: CardioTarget,
  matcher: {
    [k in CardioTarget['type']]: (val: Extract<CardioTarget, { type: k }>) => T;
  },
) {
  return match(value).with({ type: 'time' }, matcher.time).with({ type: 'distance' }, matcher.distance).exhaustive();
}

export class CardioExerciseSetBlueprint {
  constructor(
    readonly target: CardioTarget,
    readonly trackDuration: boolean,
    readonly trackDistance: boolean,
    readonly trackResistance: boolean,
    readonly trackIncline: boolean,
    readonly trackWeight: boolean,
    readonly trackSteps: boolean,
    /** Undefined when the set has no rest. Steady-state cardio does not need one; intervals do. */
    readonly restBetweenSets: Rest | undefined,
  ) {}
  static empty() {
    return new CardioExerciseSetBlueprint(
      {
        type: 'time',
        value: Duration.ofMinutes(30),
      },
      false,
      true,
      false,
      false,
      false,
      false,
      undefined,
    );
  }

  static fromJSON(json: CardioExerciseSetBlueprintJSON): CardioExerciseSetBlueprint {
    return new CardioExerciseSetBlueprint(
      fromCardioTargetJSON(json.target),
      json.trackDuration,
      json.trackDistance,
      json.trackResistance,
      json.trackIncline,
      json.trackWeight,
      json.trackSteps,
      json.restBetweenSets ? Rest.fromJSON(json.restBetweenSets) : undefined,
    );
  }

  toJSON(): CardioExerciseSetBlueprintJSON {
    return {
      target: toCardioTargetJSON(this.target),
      trackDistance: this.trackDistance,
      trackDuration: this.trackDuration,
      trackIncline: this.trackIncline,
      trackResistance: this.trackResistance,
      trackWeight: this.trackWeight,
      trackSteps: this.trackSteps,
      restBetweenSets: this.restBetweenSets ? Rest.toJSON(this.restBetweenSets) : undefined,
    };
  }

  equals(other: CardioExerciseSetBlueprint | undefined): boolean {
    if (!other) {
      return false;
    }
    return (
      this.trackDistance === other.trackDistance &&
      this.trackDuration === other.trackDuration &&
      this.trackIncline === other.trackIncline &&
      this.trackResistance === other.trackResistance &&
      this.trackWeight === other.trackWeight &&
      this.trackSteps === other.trackSteps &&
      restEquals(this.restBetweenSets, other.restBetweenSets) &&
      cardioTargetEquals(this.target, other.target)
    );
  }

  with(other: Partial<CardioExerciseSetBlueprint>): CardioExerciseSetBlueprint {
    return new CardioExerciseSetBlueprint(
      other.target ?? this.target,
      other.trackDuration ?? this.trackDuration,
      other.trackDistance ?? this.trackDistance,
      other.trackResistance ?? this.trackResistance,
      other.trackIncline ?? this.trackIncline,
      other.trackWeight ?? this.trackWeight,
      other.trackSteps ?? this.trackSteps,
      'restBetweenSets' in other ? other.restBetweenSets : this.restBetweenSets,
    );
  }
}

export class CardioExerciseBlueprint {
  readonly type = 'CardioExerciseBlueprint';
  constructor(
    readonly name: string,
    readonly sets: CardioExerciseSetBlueprint[],
    readonly notes: string,
    readonly link: string,
  ) {
    if (!sets.length) {
      throw new Error('Must have at least one set in cardio exercise');
    }
  }

  static empty() {
    return new CardioExerciseBlueprint('', [CardioExerciseSetBlueprint.empty()], '', '');
  }

  static fromJSON(json: CardioExerciseBlueprintJSON): CardioExerciseBlueprint {
    return new CardioExerciseBlueprint(
      json.name,
      json.sets.map((x) => CardioExerciseSetBlueprint.fromJSON(x)),
      json.notes,
      json.link,
    );
  }

  /** See {@link MovementKey}. */
  movementKey(): MovementKey {
    return movementKeyFor(this.name, this.type);
  }

  /** See {@link ProgressionKey}. Distance and time work are separate lineages. */
  progressionKey(): ProgressionKey {
    return `${this.name}_${this.type}_${this.sets[0]?.target.type ?? 'distance'}` as ProgressionKey;
  }

  equals(other: ExerciseBlueprint | undefined) {
    if (!other) {
      return false;
    }
    if (other === this) {
      return true;
    }
    if ('type' in other && other.type !== this.type) {
      return false;
    }
    return (
      this.name === other.name &&
      this.sets.length === other.sets.length &&
      this.sets.every((set, index) => set.equals(other.sets[index])) &&
      this.notes === other.notes &&
      this.link === other.link
    );
  }

  toJSON(): CardioExerciseBlueprintJSON {
    return {
      type: 'CardioExerciseBlueprint',
      name: this.name,
      sets: this.sets.map((x) => x.toJSON()),
      notes: this.notes,
      link: this.link,
    };
  }

  with(other: Partial<CardioExerciseBlueprint>): CardioExerciseBlueprint {
    return new CardioExerciseBlueprint(
      other.name ?? this.name,
      other.sets ?? this.sets,
      other.notes ?? this.notes,
      other.link ?? this.link,
    );
  }
}

/** Which number a rule moves. */
export type ProgressionAxis = 'reps' | 'load';

export type IncreaseStrategy = 'first' | 'middle' | 'last' | 'all';

/**
 * Which sets a rule moves. `lowestSets` ranks by the rule's own axis, so a reps rule picks the sets
 * with the smallest target rather than the smallest weight.
 */
export type SetScope = { type: 'allSets' } | { type: 'lowestSets'; pick: IncreaseStrategy };

/** What has to happen for a rule to fire. */
export type SuccessRule = 'allSetsMetTarget';

export interface ProgressionRuleInit {
  axis?: ProgressionAxis;
  step?: BigNumber;
  scope?: SetScope;
  ceiling?: BigNumber | undefined;
  onCeiling?: 'reset' | undefined;
  trigger?: SuccessRule;
}

/**
 * A progression rule from before weights stopped moving on their own. Plans, sessions and backups still
 * carry them, so they load and save unchanged, but nothing runs them.
 */
export class ProgressionRule {
  readonly type = 'ProgressionRule';
  constructor(
    readonly axis: ProgressionAxis,
    readonly step: BigNumber,
    readonly scope: SetScope = { type: 'allSets' },
    readonly ceiling?: BigNumber,
    readonly onCeiling?: 'reset',
    readonly trigger: SuccessRule = 'allSetsMetTarget',
  ) {}

  static of(init: ProgressionRuleInit & { axis: ProgressionAxis; step: BigNumber }): ProgressionRule {
    return new ProgressionRule(init.axis, init.step, init.scope, init.ceiling, init.onCeiling, init.trigger);
  }

  /** The rule almost every plan wants: put more on the bar once every set hit its target. */
  static load(step: BigNumber, scope: SetScope = { type: 'allSets' }): ProgressionRule {
    return new ProgressionRule('load', step, scope);
  }

  static fromJSON(json: ProgressionRuleJSON): ProgressionRule {
    return new ProgressionRule(
      json.axis,
      fromBigNumberJSON(json.step),
      json.scope.type === 'allSets' ? { type: 'allSets' } : { type: 'lowestSets', pick: json.scope.pick },
      json.ceiling === undefined ? undefined : fromBigNumberJSON(json.ceiling),
      json.onCeiling,
      json.trigger,
    );
  }

  toJSON(): ProgressionRuleJSON {
    return {
      axis: this.axis,
      step: toBigNumberJSON(this.step),
      scope: this.scope.type === 'allSets' ? { type: 'allSets' } : { type: 'lowestSets', pick: this.scope.pick },
      ...(this.ceiling === undefined ? {} : { ceiling: toBigNumberJSON(this.ceiling) }),
      ...(this.onCeiling === undefined ? {} : { onCeiling: this.onCeiling }),
      trigger: this.trigger,
    };
  }
}

/**
 * A rep target from before plans stopped prescribing reps. Stored sets and plans still carry one, so it
 * loads and saves unchanged; new sets get `0-0`.
 */
export interface RepsTarget {
  min: number;
  max: number;
}

/** One set of the plan. Only how many there are matters; see {@link RepsTarget}. */
export interface PlannedSet {
  reps: RepsTarget;
}

/** A planned set with no rep target, which is every set the app creates. */
function plannedSlots(count: number): PlannedSet[] {
  return Array.from({ length: count }, () => ({ reps: { min: 0, max: 0 } }));
}

/**
 * Where a movement's load comes from: the whole stored weight (`external`), what is added on top of
 * the lifter (`bodyweight`), or nothing at all (`none`).
 */
export const LoadBases = ['none', 'external', 'bodyweight'] as const;
export type Resistance = (typeof LoadBases)[number];

export function formatRepsTarget(target: RepsTarget): string {
  return target.min === target.max ? `${target.max}` : `${target.min}-${target.max}`;
}

/**
 * Every field of a weighted blueprint, each optional and each named. See
 * {@link WeightedExerciseBlueprint.of}.
 */
export interface WeightedExerciseBlueprintInit {
  name?: string;
  sets?: number;
  restBetweenSets?: Rest;
  supersetWithNext?: boolean;
  notes?: string;
  link?: string;
  resistance?: Resistance;
}

export class WeightedExerciseBlueprint {
  readonly type = 'WeightedExerciseBlueprint';

  constructor(
    readonly name: string,
    readonly plannedSets: PlannedSet[],
    readonly progression: ProgressionRule[],
    readonly restBetweenSets: Rest,
    readonly supersetWithNext: boolean,
    readonly notes: string,
    readonly link: string,
    readonly resistance: Resistance = 'external',
  ) {}

  /** Build a blueprint from named fields; preferred over the constructor's eight positional arguments. */
  static of(init: WeightedExerciseBlueprintInit = {}): WeightedExerciseBlueprint {
    return new WeightedExerciseBlueprint(
      init.name ?? '',
      plannedSlots(Math.max(init.sets ?? 3, 1)),
      [],
      init.restBetweenSets ?? Rest.medium,
      init.supersetWithNext ?? false,
      init.notes ?? '',
      init.link ?? '',
      init.resistance ?? 'external',
    );
  }

  static empty() {
    return WeightedExerciseBlueprint.of();
  }

  static fromJSON(json: WeightedExerciseBlueprintJSON): WeightedExerciseBlueprint {
    return new WeightedExerciseBlueprint(
      json.name,
      json.plannedSets.map((s) => ({ reps: { min: s.reps.min, max: s.reps.max } })),
      json.progression.map(ProgressionRule.fromJSON),
      Rest.fromJSON(json.restBetweenSets),
      json.supersetWithNext,
      json.notes,
      json.link,
      json.resistance,
    );
  }

  /** See {@link MovementKey}. */
  movementKey(): MovementKey {
    return movementKeyFor(this.name, this.type);
  }

  /**
   * See {@link ProgressionKey}. Still stored with every recorded exercise, so it keeps the formula it was
   * written with: the rep scheme drops out where a reps rule moved the targets or the exercise carries
   * no load.
   */
  progressionKey(): ProgressionKey {
    const repsAreProgressed = this.resistance === 'none' || this.progression.some((rule) => rule.axis === 'reps');
    const base = `${this.name}_${this.type}_${this.plannedSets.length}`;
    return (repsAreProgressed ? base : `${base}_${plannedSetsKey(this.plannedSets)}`) as ProgressionKey;
  }

  repsTargetForSet(index: number): RepsTarget {
    const target = this.plannedSets[index]?.reps ?? this.plannedSets.at(-1)?.reps ?? { min: 0, max: 0 };
    return { min: target.min, max: target.max };
  }

  /** At least one. Sets kept keep whatever they carried; sets added carry nothing. */
  withSets(value: number): WeightedExerciseBlueprint {
    const sets = Math.max(value, 1);
    return new WeightedExerciseBlueprint(
      this.name,
      [...this.plannedSets.slice(0, sets), ...plannedSlots(sets - Math.min(sets, this.plannedSets.length))],
      this.progression,
      this.restBetweenSets,
      this.supersetWithNext,
      this.notes,
      this.link,
      this.resistance,
    );
  }

  /** Compares what the plan still means: legacy rep targets, rules and failure rest are left out. */
  equals(other: ExerciseBlueprint | undefined) {
    if (!other) {
      return false;
    }
    if (other === this) {
      return true;
    }
    if ('type' in other && other.type !== this.type) {
      return false;
    }

    return (
      this.name === other.name &&
      this.plannedSets.length === other.plannedSets.length &&
      this.restBetweenSets.minRest.equals(other.restBetweenSets.minRest) &&
      this.restBetweenSets.maxRest.equals(other.restBetweenSets.maxRest) &&
      this.supersetWithNext === other.supersetWithNext &&
      this.notes === other.notes &&
      this.link === other.link &&
      this.resistance === other.resistance
    );
  }

  toJSON(): WeightedExerciseBlueprintJSON {
    return {
      type: 'WeightedExerciseBlueprint',
      name: this.name,
      plannedSets: this.plannedSets.map((s) => ({ reps: { min: s.reps.min, max: s.reps.max } })),
      progression: this.progression.map((rule) => rule.toJSON()),
      restBetweenSets: Rest.toJSON(this.restBetweenSets),
      supersetWithNext: this.supersetWithNext,
      notes: this.notes,
      link: this.link,
      resistance: this.resistance,
    };
  }

  with(other: WeightedExerciseBlueprintInit): WeightedExerciseBlueprint {
    const resized = other.sets === undefined ? this : this.withSets(other.sets);
    return new WeightedExerciseBlueprint(
      other.name ?? this.name,
      resized.plannedSets,
      this.progression,
      other.restBetweenSets ?? this.restBetweenSets,
      other.supersetWithNext ?? this.supersetWithNext,
      other.notes ?? this.notes,
      other.link ?? this.link,
      other.resistance ?? this.resistance,
    );
  }
}

/** The rep-scheme half of a load-based {@link ProgressionKey}, collapsing equal targets to one. */
function plannedSetsKey(plannedSets: PlannedSet[]): string {
  const targets = plannedSets.map((s) => s.reps);
  const first = targets[0];
  if (first && targets.every((t) => t.min === first.min && t.max === first.max)) {
    return formatRepsTarget(first);
  }
  return targets.map(formatRepsTarget).join(',');
}

/*
 * Two exercises can be "the same" in two different ways, and the answer differs depending on which
 * question is being asked. Both keys are produced by methods on the blueprint (and mirrored on the
 * recorded exercise), so whichever one you have in hand offers both side by side.
 *
 *   movementKey()    - is this the same *movement*, for aggregating history and for the weights and
 *                      reps a weighted exercise starts from?
 *   progressionKey() - is this the same *programmed slot*, so a cardio exercise's last settings load
 *                      into today's?
 *
 * They are branded so that a map keyed by one cannot be indexed by the other.
 */

/**
 * Identifies a movement across everything the user has ever logged. Blind to how the exercise is
 * programmed, and deliberately fuzzy about spelling - `Cable Flye`, `cable flies` and `Cable Flys`
 * all key alike - because stats, personal records, "recently completed" and a weighted exercise's
 * previous sets want one row per movement however the user typed it that day.
 *
 * Weighted and cardio are separate movements even under the same name: a rowing machine and a barbell
 * row share a word and nothing else, and their numbers are not comparable.
 */
export type MovementKey = string & { readonly __brand: 'MovementKey' };

/**
 * Identifies one exercise slot in a program. A cardio exercise carries its incline and resistance
 * along it, kept apart by target type. Every recorded exercise stores one, so the weighted formula,
 * which is sensitive to the rep scheme, stays as written even though nothing weighted reads it any more.
 * Split by type for the same reason as {@link MovementKey}.
 */
export type ProgressionKey = string & { readonly __brand: 'ProgressionKey' };

/**
 * The fuzzy half of {@link MovementKey}, for the callers that compare names alone - a stat row, a saved
 * exercise descriptor, neither of which knows whether it is weighted or cardio.
 */
export function normalizeExerciseName(name: string): string {
  if (!name) {
    return '';
  }
  const lowerName = name.toLowerCase().trim().replace(/flies/g, 'flys').replace(/flyes/g, 'flys');
  const withoutPlural = lowerName.endsWith('es')
    ? lowerName.slice(0, -2)
    : lowerName.endsWith('s')
      ? lowerName.slice(0, -1)
      : lowerName;

  return withoutPlural;
}

/**
 * For callers holding a name and a type but no blueprint - a route param, say. Prefer
 * `blueprint.movementKey()` wherever a blueprint is available.
 */
export function movementKeyFor(name: string, type: ExerciseBlueprint['type']): MovementKey {
  return `${normalizeExerciseName(name)}|${type}` as MovementKey;
}

export interface Rest {
  minRest: Duration;
  maxRest: Duration;
  failureRest: Duration;
}

export const Rest = {
  short: {
    minRest: Duration.ofSeconds(60),
    maxRest: Duration.ofSeconds(90),
    failureRest: Duration.ofSeconds(180),
  },
  medium: {
    minRest: Duration.ofSeconds(90),
    maxRest: Duration.ofSeconds(180),
    failureRest: Duration.ofSeconds(300),
  },
  long: {
    minRest: Duration.ofMinutes(3),
    maxRest: Duration.ofMinutes(5),
    failureRest: Duration.ofMinutes(8),
  },

  fromJSON(json: RestJSON): Rest {
    return {
      minRest: fromDurationJSON(json.minRest),
      maxRest: fromDurationJSON(json.maxRest),
      failureRest: fromDurationJSON(json.failureRest),
    };
  },

  toJSON(value: Rest): RestJSON {
    return {
      minRest: toDurationJSON(value.minRest),
      maxRest: toDurationJSON(value.maxRest),
      failureRest: toDurationJSON(value.failureRest),
    };
  },
} as const;
export const EmptyExerciseBlueprint = WeightedExerciseBlueprint.of();

export function restEquals(a: Rest | undefined, b: Rest | undefined): boolean {
  if (!a || !b) return a === b;
  return a.minRest.equals(b.minRest) && a.maxRest.equals(b.maxRest) && a.failureRest.equals(b.failureRest);
}

export function cardioTargetEquals(a: CardioTarget, b: CardioTarget): boolean {
  if (a.type !== b.type) return false;
  if (a.type === 'time' && b.type === 'time') {
    return a.value.equals(b.value);
  }
  if (a.type === 'distance' && b.type === 'distance') {
    return a.value.value.eq(b.value.value) && a.value.unit === b.value.unit;
  }
  return false;
}

function fromCardioTargetJSON(json: CardioTargetJSON): CardioTarget {
  return match(json)
    .returnType<CardioTarget>()
    .with({ type: 'distance' }, (j) => ({
      type: 'distance',
      value: fromDistanceJSON(j.value),
    }))
    .with({ type: 'time' }, (j) => ({
      type: 'time',
      value: fromDurationJSON(j.value),
    }))
    .exhaustive();
}
function toCardioTargetJSON(value: CardioTarget): CardioTargetJSON {
  return match(value)
    .returnType<CardioTargetJSON>()
    .with({ type: 'distance' }, (j) => ({
      type: 'distance',
      value: toDistanceJSON(j.value),
    }))
    .with({ type: 'time' }, (j) => ({
      type: 'time',
      value: toDurationJSON(j.value),
    }))
    .exhaustive();
}

export function fromDistanceJSON(json: DistanceJSON): Distance {
  return {
    value: fromBigNumberJSON(json.value),
    unit: json.unit,
  };
}

export function toDistanceJSON(value: Distance): DistanceJSON {
  return {
    value: toBigNumberJSON(value.value),
    unit: value.unit,
  };
}
