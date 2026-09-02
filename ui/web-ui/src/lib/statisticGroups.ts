import type { StatisticMetadata } from '@/types';

const collator = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true });

export function hasDefaultGoal(statistic: StatisticMetadata): boolean {
  return statistic.default_goal !== null && statistic.default_goal !== undefined;
}

/** Stable sort by category, then name (locale-aware, numeric). */
export function sortStatistics(statistics: StatisticMetadata[]): StatisticMetadata[] {
  return [...statistics].sort(
    (a, b) => collator.compare(a.category, b.category) || collator.compare(a.name, b.name)
  );
}

/** Splits statistics into those with an active default goal and the rest, both sorted. */
export function splitByGoal(statistics: StatisticMetadata[]): {
  withGoal: StatisticMetadata[];
  withoutGoal: StatisticMetadata[];
} {
  return {
    withGoal: sortStatistics(statistics.filter(hasDefaultGoal)),
    withoutGoal: sortStatistics(statistics.filter((s) => !hasDefaultGoal(s))),
  };
}

export interface CategoryGroup {
  category: string;
  statistics: StatisticMetadata[];
}

/** Groups (already sorted) statistics by category, preserving their order. */
export function groupByCategory(statistics: StatisticMetadata[]): CategoryGroup[] {
  const groups: CategoryGroup[] = [];
  for (const statistic of statistics) {
    const last = groups[groups.length - 1];
    if (last && last.category === statistic.category) {
      last.statistics.push(statistic);
    } else {
      groups.push({ category: statistic.category, statistics: [statistic] });
    }
  }
  return groups;
}
