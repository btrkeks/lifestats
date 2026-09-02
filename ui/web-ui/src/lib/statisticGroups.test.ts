import { describe, expect, it } from 'vitest';
import type { StatisticMetadata } from '@/types';
import { groupByCategory, splitByGoal } from './statisticGroups';

function stat(
  id: number,
  category: string,
  name: string,
  default_goal: Record<string, unknown> | null = null
): StatisticMetadata {
  return { id, name, category, statistic_type: 'IntegerAmount', default_goal };
}

describe('splitByGoal', () => {
  it('separates goal-bearing statistics and sorts by category then name', () => {
    const { withGoal, withoutGoal } = splitByGoal([
      stat(1, 'Work', 'Session2', { target_amount: 1 }),
      stat(2, 'Anki', 'Reps', null),
      stat(3, 'Health', 'Sleep', { target_amount: 8 }),
      stat(4, 'Anki', 'Backlog', null),
      stat(5, 'Work', 'Session1', { target_amount: 1 }),
    ]);

    expect(withGoal.map((s) => s.id)).toEqual([3, 5, 1]);
    expect(withoutGoal.map((s) => s.id)).toEqual([4, 2]);
  });

  it('treats a missing default_goal field as no goal', () => {
    const legacy = { id: 1, name: 'x', category: 'c', statistic_type: 'Habit' } as StatisticMetadata;
    expect(splitByGoal([legacy]).withoutGoal).toHaveLength(1);
  });
});

describe('groupByCategory', () => {
  it('groups consecutive statistics of the same category', () => {
    const groups = groupByCategory([
      stat(1, 'Anki', 'A'),
      stat(2, 'Anki', 'B'),
      stat(3, 'Work', 'C'),
    ]);
    expect(groups.map((g) => [g.category, g.statistics.length])).toEqual([
      ['Anki', 2],
      ['Work', 1],
    ]);
  });

  it('returns no groups for an empty list', () => {
    expect(groupByCategory([])).toEqual([]);
  });
});
