import { LoadContext } from '@/models/session-models';
import { isStalled, SessionPeak, setCue } from '@/models/set-progress';
import { Weight } from '@/models/weight';
import fc from 'fast-check';
import { describe, expect, it } from 'vitest';

const kg = (value: number) => new Weight(value, 'kilograms');
const lb = (value: number) => new Weight(value, 'pounds');
const external: LoadContext = { resistance: 'external' };
const none: LoadContext = { resistance: 'none' };
const bodyweight = (weight: Weight | undefined): LoadContext => ({ resistance: 'bodyweight', bodyweight: weight });

const previous = (weight: Weight, reps: number, context: LoadContext = external) => ({ weight, reps, context });

function aimAt(weight: Weight, last: ReturnType<typeof previous> | undefined, context: LoadContext = external) {
  return setCue({ current: { context, weight, status: { kind: 'unchecked' } }, previous: last });
}

function logged(
  weight: Weight,
  reps: number,
  last: ReturnType<typeof previous> | undefined,
  context: LoadContext = external,
) {
  return setCue({ current: { context, weight, status: { kind: 'checked', reps } }, previous: last });
}

describe('setCue', () => {
  it('aims for one more rep at the same weight', () => {
    expect(aimAt(kg(100), previous(kg(100), 8))).toEqual({ kind: 'target', reps: 9 });
  });

  it('aims for the fewest reps whose estimated one-rep max beats the previous set at a changed weight', () => {
    expect(aimAt(kg(62.5), previous(kg(60), 10))).toEqual({ kind: 'target', reps: 9 });
    expect(aimAt(kg(102.5), previous(kg(100), 8))).toEqual({ kind: 'target', reps: 8 });
    expect(aimAt(kg(80), previous(kg(100), 8))).toEqual({ kind: 'target', reps: 18 });
    expect(aimAt(kg(80), previous(kg(60), 10))).toEqual({ kind: 'target', reps: 1 });
  });

  it('needs one more rep than an exact tie', () => {
    // 75 × (30 + 2) = 60 × (30 + 10)
    expect(aimAt(kg(75), previous(kg(60), 10))).toEqual({ kind: 'target', reps: 3 });
    expect(logged(kg(75), 2, previous(kg(60), 10))).toEqual({ kind: 'hidden' });
    expect(logged(kg(75), 3, previous(kg(60), 10))).toEqual({ kind: 'better' });
  });

  it('shows no target above 30 reps', () => {
    expect(aimAt(kg(60), previous(kg(100), 8))).toEqual({ kind: 'hidden' });
    expect(aimAt(kg(100), previous(kg(100), 30))).toEqual({ kind: 'hidden' });
    expect(aimAt(kg(100), previous(kg(100), 29))).toEqual({ kind: 'target', reps: 30 });
  });

  it('compares across units', () => {
    expect(aimAt(lb(220.462), previous(kg(100), 8))).toEqual({ kind: 'target', reps: 9 });
    expect(aimAt(lb(225), previous(kg(100), 8))).toEqual({ kind: 'target', reps: 8 });
  });

  it('counts reps when both external weights are zero, and has nothing to say from zero to a load', () => {
    expect(aimAt(kg(0), previous(kg(0), 12))).toEqual({ kind: 'target', reps: 13 });
    expect(aimAt(kg(20), previous(kg(0), 12))).toEqual({ kind: 'hidden' });
    expect(aimAt(new Weight(25, 'nil'), previous(kg(20), 12))).toEqual({ kind: 'hidden' });
  });

  it('counts reps for a movement with no load', () => {
    expect(aimAt(Weight.NIL, previous(Weight.NIL, 12, none), none)).toEqual({ kind: 'target', reps: 13 });
    expect(logged(Weight.NIL, 13, previous(Weight.NIL, 12, none), none)).toEqual({ kind: 'better' });
    expect(logged(Weight.NIL, 12, previous(Weight.NIL, 12, none), none)).toEqual({ kind: 'hidden' });
  });

  it('adds each session its own bodyweight, assistance included', () => {
    const lastTime = (added: number) => previous(kg(added), 10, bodyweight(kg(80)));

    expect(aimAt(kg(2.5), lastTime(0), bodyweight(kg(80)))).toEqual({ kind: 'target', reps: 9 });
    expect(aimAt(kg(-10), lastTime(-20), bodyweight(kg(80)))).toEqual({ kind: 'target', reps: 5 });
  });

  it('asks one more rep of the same added weight, whatever either session weighed', () => {
    expect(aimAt(kg(0), previous(kg(0), 10, bodyweight(kg(80))), bodyweight(kg(82)))).toEqual({
      kind: 'target',
      reps: 11,
    });
    expect(aimAt(kg(0), previous(kg(0), 10, bodyweight(undefined)), bodyweight(undefined))).toEqual({
      kind: 'target',
      reps: 11,
    });
    expect(logged(kg(10), 10, previous(kg(10), 10, bodyweight(kg(78))), bodyweight(kg(82)))).toEqual({
      kind: 'hidden',
    });
  });

  it('has nothing to say about a changed bodyweight set without a bodyweight', () => {
    expect(aimAt(kg(10), previous(kg(0), 10, bodyweight(kg(80))), bodyweight(undefined))).toEqual({ kind: 'hidden' });
    expect(aimAt(kg(5), previous(kg(0), 10, bodyweight(undefined)), bodyweight(kg(80)))).toEqual({ kind: 'hidden' });
    expect(aimAt(kg(-80), previous(kg(0), 10, bodyweight(kg(80))), bodyweight(kg(80)))).toEqual({ kind: 'hidden' });
  });

  it('has nothing to say when the resistance changed', () => {
    expect(aimAt(kg(60), previous(kg(60), 10), bodyweight(kg(80)))).toEqual({ kind: 'hidden' });
    expect(logged(kg(60), 12, previous(kg(60), 10), bodyweight(kg(80)))).toEqual({ kind: 'hidden' });
  });

  it('has nothing to say without a previous set, or after one of no reps', () => {
    expect(aimAt(kg(60), undefined)).toEqual({ kind: 'hidden' });
    expect(logged(kg(60), 10, undefined)).toEqual({ kind: 'hidden' });
    expect(aimAt(kg(60), previous(kg(60), 0))).toEqual({ kind: 'hidden' });
  });

  it('marks only a logged set that beat its previous one', () => {
    expect(logged(kg(62.5), 9, previous(kg(60), 10))).toEqual({ kind: 'better' });
    expect(logged(kg(62.5), 8, previous(kg(60), 10))).toEqual({ kind: 'hidden' });
    expect(logged(kg(100), 8, previous(kg(100), 8))).toEqual({ kind: 'hidden' });
  });

  it('never marks a set of no reps', () => {
    expect(logged(kg(150), 0, previous(kg(100), 5))).toEqual({ kind: 'hidden' });
  });

  it('beats the previous set by doing the target, and not by one rep fewer', () => {
    const weight = fc
      .tuple(fc.integer({ min: 1, max: 1200 }), fc.constantFrom('kilograms' as const, 'pounds' as const))
      .map(([quarters, unit]) => new Weight(quarters / 4, unit));
    fc.assert(
      fc.property(weight, fc.integer({ min: 1, max: 30 }), weight, (before, reps, now) => {
        const last = previous(before, reps);
        const cue = aimAt(now, last);
        fc.pre(cue.kind === 'target');
        if (cue.kind !== 'target') {
          return;
        }
        expect(logged(now, cue.reps, last)).toEqual({ kind: 'better' });
        expect(logged(now, cue.reps - 1, last)).toEqual({ kind: 'hidden' });
      }),
    );
  });
});

describe('isStalled', () => {
  const loads = (...peaks: number[]): SessionPeak[] =>
    peaks.map((peak) => ({ kind: 'load', resistance: 'external', peak: kg(peak) }));
  const stalled = (
    recent: SessionPeak[],
    sets: { weight: Weight; reps: number }[] = [],
    context: LoadContext = external,
  ) => isStalled({ recent, today: { context, sets } });

  it('flags three sessions in a row without a gain', () => {
    expect(stalled(loads(100, 100, 102, 102))).toBe(true);
    expect(stalled(loads(100, 100, 102, 102, 90))).toBe(true);
  });

  it('lets a gain on the session before break the streak, even below an older best', () => {
    expect(stalled(loads(101, 100, 100, 110))).toBe(false);
    expect(stalled(loads(100, 101, 100, 100))).toBe(false);
  });

  it('needs four sessions it can compare', () => {
    expect(stalled(loads(100, 100, 100))).toBe(false);
    expect(stalled([...loads(100, 100), { kind: 'unavailable' }, ...loads(100)])).toBe(false);
    expect(stalled(loads(100, 100, 100, 100), [], bodyweight(kg(80)))).toBe(false);
  });

  it('clears once a set today beats the latest session', () => {
    expect(stalled(loads(100, 100, 100, 100), [{ weight: kg(75), reps: 10 }])).toBe(true);
    expect(stalled(loads(100, 100, 100, 100), [{ weight: kg(80), reps: 10 }])).toBe(false);
    expect(stalled(loads(100, 100, 100, 100), [{ weight: kg(200), reps: 0 }])).toBe(true);
  });

  it('compares reps for a movement with no load', () => {
    const reps = (...peaks: number[]): SessionPeak[] => peaks.map((peak) => ({ kind: 'reps', peak }));

    expect(stalled(reps(10, 10, 10, 12), [], none)).toBe(true);
    expect(stalled(reps(10, 10, 10, 12), [{ weight: Weight.NIL, reps: 11 }], none)).toBe(false);
    expect(stalled(reps(10, 10, 10, 12), [], external)).toBe(false);
  });

  it('adds today’s bodyweight to a bodyweight set', () => {
    // 90kg × 10 each time: 90 × (1 + 10 / 30) = 120
    const recent: SessionPeak[] = [120, 120, 120, 120].map((peak) => ({
      kind: 'load',
      resistance: 'bodyweight',
      peak: kg(peak),
    }));

    expect(stalled(recent, [{ weight: kg(0), reps: 10 }], bodyweight(kg(90)))).toBe(true);
    expect(stalled(recent, [{ weight: kg(2.5), reps: 10 }], bodyweight(kg(90)))).toBe(false);
    expect(stalled(recent, [{ weight: kg(2.5), reps: 10 }], bodyweight(undefined))).toBe(true);
  });
});
