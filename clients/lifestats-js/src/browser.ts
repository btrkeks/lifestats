/**
 * Browser/DataviewJS entry point
 * Exposes LifeStats client as a global IIFE for use in environments
 * without module resolution (like Obsidian DataviewJS)
 */

import { LifeStatsClient } from './client';
import type { ClientOptions } from './client';
import {
  createIntegerAmountGoal,
  createHabitGoal,
  createPointInTimeGoal,
  createTimeSpansGoal,
  createDurationGoal,
  formatDate,
  formatTime,
} from './schemas';

// Export everything as a single object for IIFE
export default {
  Client: LifeStatsClient,

  // Helper functions for creating goals
  createIntegerAmountGoal,
  createHabitGoal,
  createPointInTimeGoal,
  createTimeSpansGoal,
  createDurationGoal,

  // Date/time helpers
  formatDate,
  formatTime,

  // Convenience: Create a client instance
  createClient: (options?: ClientOptions) => new LifeStatsClient(options),
};

// For IIFE build, also expose types for TypeScript users
export type * from './types';
export type { ClientOptions } from './client';
