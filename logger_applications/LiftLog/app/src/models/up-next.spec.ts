import { describe, expect, it } from 'vitest';
import { upNextIndex } from '@/models/up-next';

const ms = (iso: string) => Date.parse(iso);

describe('upNextIndex', () => {
  it('picks the workout finished longest ago', () => {
    const lastFinished = new Map([
      ['DAY 1', ms('2026-10-10T18:00:00+02:00')],
      ['DAY 2', ms('2026-10-08T18:00:00+02:00')],
    ]);
    expect(upNextIndex(['DAY 1', 'DAY 2'], lastFinished)).toBe(1);
  });

  it('orders two workouts finished on the same day by time', () => {
    const lastFinished = new Map([
      ['DAY 1', ms('2026-10-10T19:30:00+02:00')],
      ['DAY 2', ms('2026-10-10T09:00:00+02:00')],
    ]);
    expect(upNextIndex(['DAY 1', 'DAY 2'], lastFinished)).toBe(1);
  });

  it('puts a workout never finished first', () => {
    const lastFinished = new Map([
      ['A', ms('2020-01-01T10:00:00Z')],
      ['C', ms('2026-10-10T10:00:00Z')],
    ]);
    expect(upNextIndex(['A', 'B', 'C'], lastFinished)).toBe(1);
  });

  it('breaks a tie by plan order', () => {
    expect(upNextIndex(['A', 'B', 'C'], new Map())).toBe(0);
    const sameTime = ms('2026-10-10T10:00:00Z');
    expect(
      upNextIndex(
        ['A', 'B', 'C'],
        new Map([
          ['A', ms('2026-10-11T10:00:00Z')],
          ['B', sameTime],
          ['C', sameTime],
        ]),
      ),
    ).toBe(1);
  });

  it('counts the workout in progress as done now', () => {
    const lastFinished = new Map([
      ['DAY 1', ms('2026-10-08T18:00:00+02:00')],
      ['DAY 2', ms('2026-10-10T18:00:00+02:00')],
    ]);
    expect(upNextIndex(['DAY 1', 'DAY 2'], lastFinished, 'DAY 1')).toBe(1);
  });

  it('chooses nothing for a plan with one workout', () => {
    expect(upNextIndex(['Full body'], new Map())).toBeUndefined();
    expect(upNextIndex(['Full body', 'Legs'], new Map())).toBe(0);
  });
});
