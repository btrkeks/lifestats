import { Resistance } from '@/models/blueprint-models';
import { CompletedWeightedSet, LoadContext } from '@/models/session-models';
import { Weight } from '@/models/weight';
import BigNumber from 'bignumber.js';

/** What a set row says beside its Previous: the reps to aim for, or that the logged set beat it. */
export type SetCue = { kind: 'hidden' } | { kind: 'target'; reps: number } | { kind: 'better' };

/** A movement's best in one finished session: its best estimated one-rep max, or its most reps when it has no load. */
export type SessionPeak =
  | { kind: 'load'; resistance: 'external' | 'bodyweight'; peak: Weight }
  | { kind: 'reps'; peak: number }
  | { kind: 'unavailable' };

/** How many of a movement's latest sessions the stall rule reads: one to compare against, then three without a gain. */
export const stallWindow = 4;

/** Beyond this the Epley estimate is too loose to aim at. */
const maxTargetReps = 30;

/** Reps alone decide when both sides moved the same load; otherwise the Epley estimate does. */
type Comparison =
  | { kind: 'reps'; previousReps: number }
  | { kind: 'load'; previousScore: BigNumber; currentLoad: BigNumber };

/**
 * Unchecked: the fewest reps at the row's weight that beat its Previous. Checked: whether it did. The
 * target is only ever shown: ✓ still logs the Previous reps.
 */
export function setCue(input: {
  current: {
    context: LoadContext;
    weight: Weight;
    status: { kind: 'unchecked' } | { kind: 'checked'; reps: number };
  };
  previous: CompletedWeightedSet | undefined;
}): SetCue {
  const { current, previous } = input;
  const compared = previous && compare(previous, current);
  if (!previous || !compared) {
    return { kind: 'hidden' };
  }
  if (current.status.kind === 'checked') {
    return current.status.reps > 0 && beats(compared, current.status.reps) ? { kind: 'better' } : { kind: 'hidden' };
  }
  if (previous.reps === 0) {
    return { kind: 'hidden' };
  }
  const reps = targetReps(compared);
  return reps <= maxTargetReps ? { kind: 'target', reps } : { kind: 'hidden' };
}

/**
 * Whether the movement's best set has not improved over its last three sessions. `recent` is newest
 * first. Today's checked sets lift the flag as soon as one beats the latest session.
 */
export function isStalled(input: {
  recent: readonly SessionPeak[];
  today: { context: LoadContext; sets: readonly { weight: Weight; reps: number }[] };
}): boolean {
  const { resistance } = input.today.context;
  const scores = input.recent.slice(0, stallWindow).map((peak) => peakScore(peak, resistance));
  if (scores.length < stallWindow || !scores.every((score): score is BigNumber => score !== undefined)) {
    return false;
  }
  // Newest first, so a gain is a score above the one after it.
  const gained = scores.some((score, index) => {
    const older = scores[index + 1];
    return older !== undefined && score.isGreaterThan(older);
  });
  const [latest] = scores;
  if (gained || !latest) {
    return false;
  }
  return !input.today.sets.some((set) => setScore(input.today.context, set)?.isGreaterThan(latest));
}

function compare(
  previous: CompletedWeightedSet,
  current: { context: LoadContext; weight: Weight },
): Comparison | undefined {
  if (previous.context.resistance !== current.context.resistance) {
    return undefined;
  }
  // The same weight on the bar or belt is one more rep, whatever the scale said: a drifting or missing
  // bodyweight would otherwise turn a repeat into progress, or hide the target altogether.
  if (current.context.resistance === 'none' || current.weight.equals(previous.weight, true)) {
    return { kind: 'reps', previousReps: previous.reps };
  }
  const before = effectiveLoad(previous.context, previous.weight);
  const now = effectiveLoad(current.context, current.weight);
  if (!before || !now) {
    return undefined;
  }
  // Epley needs a positive load on both sides, and a set of no reps has no estimate to beat.
  if (!before.value.isGreaterThan(0) || !now.value.isGreaterThan(0) || previous.reps === 0) {
    return undefined;
  }
  return {
    kind: 'load',
    previousScore: before.value.multipliedBy(30 + previous.reps),
    currentLoad: now.convertTo(before.unit).value,
  };
}

/** Epley's `load × (1 + reps / 30)`, with both sides multiplied by 30 so it stays exact. */
function beats(compared: Comparison, reps: number): boolean {
  return compared.kind === 'reps'
    ? reps > compared.previousReps
    : compared.currentLoad.multipliedBy(30 + reps).isGreaterThan(compared.previousScore);
}

function targetReps(compared: Comparison): number {
  if (compared.kind === 'reps') {
    return compared.previousReps + 1;
  }
  const estimate = compared.previousScore
    .dividedBy(compared.currentLoad)
    .minus(30)
    .integerValue(BigNumber.ROUND_FLOOR)
    .toNumber();
  let reps = Math.max(1, estimate + 1);
  // The division rounds, so settle on the exact boundary with the comparison itself.
  while (reps > 1 && beats(compared, reps - 1)) {
    reps--;
  }
  while (!beats(compared, reps)) {
    reps++;
  }
  return reps;
}

/** The load a set moved, or undefined when it cannot be known. */
function effectiveLoad(context: LoadContext, weight: Weight): Weight | undefined {
  switch (context.resistance) {
    case 'none':
      return undefined;
    case 'external':
      return weight.unit === 'nil' ? undefined : weight;
    case 'bodyweight': {
      const { bodyweight } = context;
      if (!bodyweight || bodyweight.unit === 'nil' || !bodyweight.value.isGreaterThan(0)) {
        return undefined;
      }
      const total = bodyweight.plus(weight);
      return total.value.isGreaterThan(0) ? total : undefined;
    }
    default: {
      const _exhaustive: never = context;
      return _exhaustive;
    }
  }
}

function peakScore(peak: SessionPeak, resistance: Resistance): BigNumber | undefined {
  switch (peak.kind) {
    case 'load':
      return peak.resistance === resistance ? peak.peak.convertTo('kilograms').value : undefined;
    case 'reps':
      return resistance === 'none' ? new BigNumber(peak.peak) : undefined;
    case 'unavailable':
      return undefined;
    default: {
      const _exhaustive: never = peak;
      return _exhaustive;
    }
  }
}

/** A set today on the same scale as {@link peakScore}: estimated one-rep max in kilograms, or reps. */
function setScore(context: LoadContext, set: { weight: Weight; reps: number }): BigNumber | undefined {
  if (context.resistance === 'none') {
    return new BigNumber(set.reps);
  }
  const load = set.reps > 0 ? effectiveLoad(context, set.weight) : undefined;
  return load
    ?.convertTo('kilograms')
    .value.multipliedBy(30 + set.reps)
    .dividedBy(30);
}
