import { describe, expect, it } from 'vitest';
import { formatClock, formatDuration, formatGoal } from './goalFormat';

describe('formatClock', () => {
  it('drops zero seconds', () => {
    expect(formatClock('07:45:00')).toBe('07:45');
  });
  it('keeps non-zero seconds and HH:MM input', () => {
    expect(formatClock('07:45:30')).toBe('07:45:30');
    expect(formatClock('07:45')).toBe('07:45');
  });
});

describe('formatDuration', () => {
  it('formats hours, minutes and seconds', () => {
    expect(formatDuration(5400)).toBe('1h 30m');
    expect(formatDuration(45)).toBe('45s');
    expect(formatDuration(0)).toBe('0s');
    expect(formatDuration(-60)).toBe('-1m');
  });
});

describe('formatGoal', () => {
  it('formats amount goals with a comparison sign', () => {
    expect(
      formatGoal({ type: 'IntegerAmount', data: { target: 27000, is_at_least: true } })
    ).toBe('≥ 27,000');
    expect(formatGoal({ type: 'Summing', data: { target: 3600, is_at_least: false } })).toBe(
      '≤ 3,600'
    );
  });
  it('formats time difference goals as durations', () => {
    expect(
      formatGoal({ type: 'TimeDifference', data: { target: 5400, is_at_least: true } })
    ).toBe('≥ 1h 30m');
  });
  it('formats habit goals', () => {
    expect(formatGoal({ type: 'Habit', data: { occurred: true } })).toBe('Done');
    expect(formatGoal({ type: 'Habit', data: { occurred: false } })).toBe('Not done');
  });
  it('formats point-in-time goals as deadlines', () => {
    expect(formatGoal({ type: 'PointInTime', data: { deadline: '22:00:00' } })).toBe('by 22:00');
  });
  it('formats time spans goals as frame list with efficiency', () => {
    expect(
      formatGoal({
        type: 'TimeSpans',
        data: {
          target: [
            { start: '07:45:00', end: '08:45:00' },
            { start: '12:30', end: '14:00' },
          ],
          efficiency: 90,
        },
      })
    ).toBe('07:45–08:45, 12:30–14:00 · 90% eff.');
  });
});
