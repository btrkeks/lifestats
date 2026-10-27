import { describe, expect, it } from 'vitest';
import type { StatisticMetadata } from '@/types';
import { highlightParts, searchStatistics } from './search';

function statistic(id: number, name: string, category: string): StatisticMetadata {
  return { id, name, category, statistic_type: 'IntegerAmount', default_goal: null };
}

const statistics = [
  statistic(23, '読書時間', '夜ルーティン'),
  statistic(110, '[Ebook] Characters Read', '読書'),
  statistic(134, '[Yomikata] Cards Reviewed', 'Yomikata'),
];

describe('searchStatistics', () => {
  it('matches names and categories', () => {
    expect(searchStatistics(statistics, '読書').map((s) => s.id)).toEqual([23, 110]);
    expect(searchStatistics(statistics, 'yomi').map((s) => s.id)).toEqual([134]);
  });

  it('returns everything for a blank query', () => {
    expect(searchStatistics(statistics, '  ')).toHaveLength(3);
  });
});

describe('highlightParts', () => {
  it('marks every occurrence, ignoring case', () => {
    expect(highlightParts('Reps and reps', 'REPS')).toEqual([
      { text: 'Reps', match: true },
      { text: ' and ', match: false },
      { text: 'reps', match: true },
    ]);
  });

  it('marks Japanese text', () => {
    expect(highlightParts('読書時間', '読書')).toEqual([
      { text: '読書', match: true },
      { text: '時間', match: false },
    ]);
  });
});
