import { describe, expect, it } from 'vitest';
import { formatSeconds, formatValue, formatValueWithUnit, measureOf } from './measure';

const amount = (name: string) => measureOf({ name, statistic_type: 'IntegerAmount' });

describe('measureOf', () => {
  it('reads count units from the statistic name', () => {
    expect(amount('[Clef] Piano practice (minutes)')).toEqual({ kind: 'count', unit: 'min' });
    expect(amount('[Anki] Reps (日本語::4 - Sentences)')).toEqual({ kind: 'count', unit: 'reps' });
    expect(amount('[Yomikata] Cards Reviewed')).toEqual({ kind: 'count', unit: 'cards' });
    expect(amount('[Ebook] Characters Read')).toEqual({ kind: 'count', unit: 'chars' });
    expect(amount('Total Outreach')).toEqual({ kind: 'count', unit: null });
  });

  it('treats duration-named amounts as seconds', () => {
    expect(amount('[Fitbit] Sleep Duration')).toEqual({ kind: 'duration' });
    expect(amount('ブラウザーを利用した時間')).toEqual({ kind: 'duration' });
    expect(amount('Distracting Screen Time')).toEqual({ kind: 'duration' });
  });

  it('keeps an explicit minutes unit over the duration name', () => {
    expect(amount('[Yomikata] Study Time (minutes)')).toEqual({ kind: 'count', unit: 'min' });
  });

  it('maps the other statistic types', () => {
    expect(measureOf({ name: '就寝時間', statistic_type: 'PointInTime' })).toEqual({
      kind: 'clock',
    });
    expect(measureOf({ name: '読書時間', statistic_type: 'TimeSpans' })).toEqual({ kind: 'spans' });
    expect(measureOf({ name: 'x', statistic_type: 'Habit' })).toEqual({ kind: 'habit' });
  });
});

describe('formatting', () => {
  it('formats seconds compactly', () => {
    expect(formatSeconds(25200)).toBe('7 h');
    expect(formatSeconds(5915)).toBe('1 h 39 min');
    expect(formatSeconds(2700)).toBe('45 min');
    expect(formatSeconds(20)).toBe('20 s');
  });

  it('formats values per measure', () => {
    expect(formatValue({ kind: 'count', unit: 'cards' }, 4000)).toBe('4,000');
    expect(formatValueWithUnit({ kind: 'count', unit: 'chars' }, 4000)).toBe('4,000 chars');
    expect(formatValue({ kind: 'clock' }, '22:37:00')).toBe('22:37');
    expect(formatValue({ kind: 'habit' }, true)).toBe('Done');
    expect(formatValue({ kind: 'spans' }, 3600)).toBe('1 h');
  });
});

describe('TimedAmounts measures', () => {
  it('recognizes mg and never guesses duration from the name', () => {
    const measure = measureOf({ name: '[OpenNutriTracker] Caffeine (mg)', statistic_type: 'TimedAmounts' });
    expect(measure).toEqual({ kind: 'count', unit: 'mg' });
    expect(formatValueWithUnit(measure, 180)).toBe('180 mg');
    expect(formatValueWithUnit(measure, 0)).toBe('0 mg');
    expect(measureOf({ name: 'Duration', statistic_type: 'TimedAmounts' })).toEqual({ kind: 'count', unit: null });
    expect(measureOf({ name: 'Nicotine (MG)', statistic_type: 'TimedAmounts' })).toEqual({ kind: 'count', unit: 'mg' });
  });
});
