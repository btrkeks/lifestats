// EXCERPT — clients/lifestats-js/src/types.ts
// Remove the second hand-written list; derive from the Zod enum.
import type { z } from 'zod';
import type { statisticTypeSchema } from './schemas';
export type StatisticType = z.infer<typeof statisticTypeSchema>;
// (check for an import cycle schemas.ts → types.ts; if present, move the enum to its own module)
