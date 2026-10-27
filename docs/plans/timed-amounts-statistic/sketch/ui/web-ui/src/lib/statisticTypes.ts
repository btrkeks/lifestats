// EXCERPT — ui/web-ui/src/lib/statisticTypes.ts
// `derived: boolean` cannot say "source-owned"; with only that flag, TimedAmounts
// would appear in CreateStatisticDialog (LOGGED_TYPES = !derived) and fail on POST.
// Replace the flag with an exhaustive origin; every entry must choose (CE).
export interface StatisticTypeInfo {
  type: StatisticType;
  label: string;
  title: string;
  description: string;
  icon: LucideIcon;
  origin: 'logged' | 'derived' | 'source'; // was: derived: boolean
}

export const STATISTIC_TYPES: Record<StatisticType, StatisticTypeInfo> = {
  // ... existing entries: derived:false → origin:'logged', derived:true → origin:'derived'
  TimedAmounts: {
    type: 'TimedAmounts',
    label: 'Intake',
    title: 'Timed amounts',
    description: 'Amounts with a time, synced from an app (caffeine, nicotine)',
    icon: Coffee,
    origin: 'source',
  },
};

export const LOGGED_TYPES = Object.values(STATISTIC_TYPES).filter((i) => i.origin === 'logged');
export const DERIVED_TYPES = Object.values(STATISTIC_TYPES).filter((i) => i.origin === 'derived');
// canEditGoals: unchanged → TimedAmounts goals are editable (daily total cap/minimum).
// The only other `.derived` reader is CreateStatisticDialog.tsx:210
//   const derived = STATISTIC_TYPES[type].origin === 'derived';   (CE once the field is gone)
// The web UI has no manual log-value action (no /log/ calls), so nothing else to hide.
