# @lifestats/client

TypeScript client library for the LifeStats API.

## Features

- 🎯 **Lean & focused**: Core CRUD operations for statistics and goals
- 🔒 **Type-safe**: Full TypeScript support with runtime validation via Zod
- 📦 **Multi-format**: ESM, CommonJS, and IIFE builds for maximum compatibility
- 🌐 **Universal**: Works in Node.js, browsers, and Obsidian DataviewJS
- 🚫 **Zero dependencies** (except Zod for validation)

## Installation

```bash
# npm
npm install @lifestats/client

# pnpm
pnpm add @lifestats/client

# yarn
yarn add @lifestats/client
```

## Quick Start

```typescript
import { LifeStatsClient, createIntegerAmountGoal } from '@lifestats/client';

// Create a client
const client = new LifeStatsClient({
  baseUrl: 'http://localhost:3000',
});

// Get all statistics
const stats = await client.getAllStatistics();

// Log a value
await client.logValue(1, '2025-01-15', {
  value: '10',
  update_policy: 'sum',
});

// Set a goal
await client.setDefaultGoal(1, {
  goal_data: createIntegerAmountGoal(100, true),
});
```

## Usage by Environment

### 1. Web UI / Next.js / React

```typescript
import { LifeStatsClient } from '@lifestats/client';

const client = new LifeStatsClient({
  baseUrl: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3000',
});

// Use in React components
function StatsPage() {
  const [stats, setStats] = useState([]);

  useEffect(() => {
    client.getAllStatistics().then(setStats);
  }, []);

  return <div>{/* render stats */}</div>;
}
```

### 2. Browser Extension (Chrome/Firefox)

```typescript
import { LifeStatsClient } from '@lifestats/client';

const client = new LifeStatsClient({
  baseUrl: 'http://localhost:3000',
});

// Log screen time every 20 seconds
setInterval(async () => {
  const date = new Date().toISOString().split('T')[0];
  await client.logValue(16, date, {
    value: '20',
    update_policy: 'sum',
  });
}, 20000);
```

### 3. Obsidian DataviewJS

First, copy `dist/browser/lifestats.js` to your vault (e.g., `Scripts/lifestats.js`).

Then create a reusable template:

````markdown
```dataviewjs
// Load the client (once per note)
const loadLifeStats = async () => {
  if (window.LifeStats) return window.LifeStats;

  const script = await app.vault.adapter.read('Scripts/lifestats.js');
  eval(script);
  return window.LifeStats;
};

const LifeStats = await loadLifeStats();

// Create a client instance
const client = LifeStats.createClient({
  baseUrl: 'http://localhost:3000',
});

// Fetch and display statistics
const stats = await client.getAllStatistics();

dv.table(
  ["Statistic", "Type", "Category"],
  stats.map(s => [s.name, s.statistic_type, s.category])
);

// Get goal for a specific statistic
const goal = await client.getDefaultGoal(1);
if (goal) {
  dv.paragraph(`Goal: ${JSON.stringify(goal.goal_data)}`);
}
```
````

### 4. Node.js / CLI Logger

```typescript
import { LifeStatsClient } from '@lifestats/client';

const client = new LifeStatsClient({
  baseUrl: 'http://localhost:3000',
});

// Log from command line
async function logActivity(statisticId: number, value: string) {
  const today = new Date().toISOString().split('T')[0];
  await client.logValue(statisticId, today, {
    value,
    update_policy: 'sum',
  });
  console.log(`Logged ${value} to statistic ${statisticId}`);
}

// Run
logActivity(5, '100');
```

## API Reference

### Client Configuration

```typescript
const client = new LifeStatsClient({
  baseUrl: 'http://localhost:3000', // API base URL
  headers: {}, // Optional custom headers
  validateResponses: true, // Enable Zod validation (default: true)
});
```

### Statistics Methods

#### `getAllStatistics()`
Get all statistics.

```typescript
const stats = await client.getAllStatistics();
// Returns: StatisticMetadata[]
```

#### `getStatistic(id: number)`
Get a single statistic by ID.

```typescript
const stat = await client.getStatistic(1);
// Returns: StatisticMetadata
```

#### `createStatistic(request: CreateStatisticRequest)`
Create a new statistic.

```typescript
const newStat = await client.createStatistic({
  name: 'Push Ups',
  category: 'Fitness',
  statistic_type: 'IntegerAmount',
});
// Returns: StatisticMetadata
```

#### `deleteStatistic(id: number)`
Delete a statistic.

```typescript
await client.deleteStatistic(1);
```

#### `logValue(statisticId: number, date: string, request: LogValueRequest)`
Log a value to a statistic.

```typescript
await client.logValue(1, '2025-01-15', {
  value: '10',
  update_policy: 'sum', // 'sum' | 'overwrite' | 'max' | 'min'
});
```

#### `logTimeFrame(statisticId: number, date: string, request: LogTimeFrameRequest)`
Log a time frame to a TimeSpans statistic.

```typescript
await client.logTimeFrame(2, '2025-01-15', {
  start_time: '09:00',
  end_time: '17:00',
  update_policy: 'sum',
});
```

### Goals Methods

#### `getDefaultGoal(statisticId: number)`
Get default goal for a statistic. Returns `null` if no goal is set.

```typescript
const goal = await client.getDefaultGoal(1);
// Returns: GoalResponse | null
```

#### `setDefaultGoal(statisticId: number, request: SetDefaultGoalRequest)`
Set default goal for a statistic.

```typescript
import { createIntegerAmountGoal } from '@lifestats/client';

await client.setDefaultGoal(1, {
  goal_data: createIntegerAmountGoal(100, true),
});
```

#### `deleteDefaultGoal(statisticId: number)`
Delete default goal for a statistic.

```typescript
await client.deleteDefaultGoal(1);
```

#### `getDailyGoal(statisticId: number, date: string)`
Get daily goal for a specific date. Returns `null` if no goal is set.

```typescript
const goal = await client.getDailyGoal(1, '2025-01-15');
// Returns: GoalResponse | null
```

#### `setDailyGoal(statisticId: number, date: string, request: SetDailyGoalRequest)`
Set daily goal for a specific date. Can either set a goal or suppress the default.

```typescript
// Set a goal
await client.setDailyGoal(1, '2025-01-15', {
  kind: 'goal',
  goal_data: createIntegerAmountGoal(150, true),
});

// Or suppress the default goal for this day
await client.setDailyGoal(1, '2025-01-15', {
  kind: 'suppressed',
});
```

#### `deleteDailyGoal(statisticId: number, date: string)`
Delete daily goal for a specific date.

```typescript
await client.deleteDailyGoal(1, '2025-01-15');
```

## Goal Helpers

The library provides type-safe helper functions for creating goal data:

### `createIntegerAmountGoal(target: number, isAtLeast: boolean)`
For IntegerAmount statistics.

```typescript
const goalData = createIntegerAmountGoal(100, true);
// Returns: { target: "100", is_at_least: "true" }
```

### `createHabitGoal(occurred: boolean)`
For Habit statistics.

```typescript
const goalData = createHabitGoal(true);
// Returns: { occurred: "true" }
```

### `createPointInTimeGoal(deadline: string)`
For PointInTime statistics.

```typescript
const goalData = createPointInTimeGoal('23:00');
// Returns: { deadline: "23:00" }
```

### `createTimeSpansGoal(targetFrames: Array<{start: string, end: string}>, efficiency?: number)`
For TimeSpans statistics.

```typescript
const goalData = createTimeSpansGoal(
  [{ start: '09:00:00', end: '17:00:00' }],
  80
);
// Returns: { target: "[{\"start\":\"09:00:00\",\"end\":\"17:00:00\"}]", efficiency: "80" }
```

### `createDurationGoal(targetSeconds: number, isAtLeast: boolean)`
For Summing and TimeDifference statistics.

```typescript
const goalData = createDurationGoal(3600, true);
// Returns: { target: "3600", is_at_least: "true" }
```

## Date/Time Helpers

### `formatDate(date: Date): string`
Format a Date object as `YYYY-MM-DD`.

```typescript
const dateStr = formatDate(new Date());
// Returns: "2025-01-15"
```

### `formatTime(date: Date): string`
Format a Date object as `HH:MM:SS`.

```typescript
const timeStr = formatTime(new Date());
// Returns: "14:30:00"
```

### `validateDateString(dateStr: string): boolean`
Check if a string is in `YYYY-MM-DD` format.

```typescript
const isValid = validateDateString('2025-01-15');
// Returns: true
```

## Error Handling

All methods throw `LifeStatsError` on API errors:

```typescript
import { LifeStatsError } from '@lifestats/client';

try {
  await client.getStatistic(999);
} catch (error) {
  if (error instanceof LifeStatsError) {
    console.error(`API Error ${error.statusCode}: ${error.message}`);
    console.error('Response:', error.response);
  }
}
```

## TypeScript Types

The library exports all types:

```typescript
import type {
  StatisticType,
  UpdatePolicy,
  StatisticMetadata,
  CreateStatisticRequest,
  LogValueRequest,
  LogTimeFrameRequest,
  GoalData,
  SetDefaultGoalRequest,
  SetDailyGoalRequest,
  GoalResponse,
  DateString,
  TimeString,
} from '@lifestats/client';
```

## Validation

By default, all requests and responses are validated with Zod schemas. You can disable this for performance:

```typescript
const client = new LifeStatsClient({
  baseUrl: 'http://localhost:3000',
  validateResponses: false, // Disable validation
});
```

## Build Outputs

The library provides three build formats:

- **ESM** (`dist/index.js`) - For modern bundlers and Node.js
- **CommonJS** (`dist/index.cjs`) - For older Node.js environments
- **IIFE** (`dist/browser/lifestats.js`) - For browser/DataviewJS usage

## Development

```bash
# Install dependencies
pnpm install

# Type check
pnpm type-check

# Build
pnpm build

# Watch mode
pnpm dev
```

## License

MIT
