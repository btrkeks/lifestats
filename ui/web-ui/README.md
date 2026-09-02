# LifeStats Web UI

A modern Next.js web application for managing statistics and goals in the LifeStats personal observability platform.

## Overview

The Web UI is the **goal management interface** for LifeStats. Its primary job is to make excusing a missed day painless: the dashboard lists every statistic with an active goal, shows its last 14 days as a strip, and lets you suppress (or override) the goal for any day with one click. It also creates statistics and edits default goals.

**Tech Stack:**
- **Framework**: Next.js 15.5.4 (App Router)
- **Language**: TypeScript 5
- **UI**: React 19.1.0 + Tailwind CSS 4
- **State Management**: TanStack Query (React Query) v5
- **Form Handling**: React Hook Form v7 + Zod validation
- **HTTP Client**: Axios v1.12.2

## Features

### What This UI Provides

- **Goal Dashboard** (`/`): Statistics with an active default goal come first, grouped by category, each with a 14-day achievement strip (achieved / missed / in progress / suppressed / no goal). Statistics without a goal sit in a collapsed list below with a "Set goal" action.
- **One-click day actions**: Clicking a day in the strip opens a popover to suppress the goal for that day, set a custom goal for it, or remove an existing override. No confirmation dialogs; every action is reversible.
- **Goal Editor**: Modal with the default goal and the daily override for a chosen date. The override state (not set / suppressed / custom goal) always reflects the server, so it survives reloads.
- **Statistic Creation** (`/statistics/new`): Create new statistics with type-specific configuration
- **Type-Safe Forms**: Polymorphic forms that adapt to different statistic types with validation

### What This UI Does NOT (Yet) Provide

- **Data Logging**: No UI for logging values to statistics (use logger applications or API directly)
- **Plot Visualization**: No integration with plot rendering endpoints
- **Authentication**: Open access to all features

## Getting Started

### Prerequisites

- **Node.js**: v18.17 or later recommended
- **LifeStats Backend**: The Rust API server must be running (see `CLAUDE.md` in project root)

### Installation

```bash
cd ui/web-ui
npm install
```

### Development

```bash
npm run dev
```

This starts the development server on `http://localhost:3001` (the backend owns port 3000, and the production UI service also listens on 3001, so stop it first or pass another port: `npm run dev -- -p 3002`).

To point the UI at a backend on another port, set `NEXT_PUBLIC_API_URL`:

```bash
# .env.local, or inline: NEXT_PUBLIC_API_URL=http://localhost:3010 npm run dev
NEXT_PUBLIC_API_URL=http://localhost:3000
```

### Production Build

```bash
npm run build
npm start
```

## Project Structure

```
ui/web-ui/
├── src/
│   ├── app/                          # Next.js App Router pages
│   │   ├── page.tsx                  # Goal dashboard (/)
│   │   ├── layout.tsx                # Root layout: providers + AppShell
│   │   └── statistics/new/           # Statistic creation flow
│   │
│   ├── components/                   # Generic UI: AppShell, ConfirmButton, MutationStatus
│   │
│   ├── features/                     # Feature-based components
│   │   ├── statistics/
│   │   │   ├── components/           # GoalCard, StatisticRow, CreateStatisticForm, ...
│   │   │   └── hooks/                # useStatistics
│   │   └── goals/
│   │       ├── components/           # DayStrip, DayPopover, DayLegend, GoalEditorModal,
│   │       │   │                     #   DefaultGoalEditor, DailyGoalOverride, GoalForm
│   │       │   └── fields/           # Type-specific goal input fields
│   │       ├── dayStatus.ts          # classifyDay(): achievement → cell status
│   │       └── hooks/                # useGoals (queries, mutations, cache invalidation)
│   │
│   ├── lib/                          # Core utilities
│   │   ├── api.ts                    # API client (axios wrapper)
│   │   ├── schemas.ts                # Zod schemas: form validation + response parsing
│   │   ├── goalFormat.ts             # formatGoal(): human-readable goal summaries
│   │   ├── statisticGroups.ts        # splitByGoal() / groupByCategory()
│   │   ├── dates.ts                  # ISO date helpers (date-fns)
│   │   ├── constants.ts              # Statistic type definitions & icons
│   │   └── providers.tsx             # React Query provider setup
│   │
│   └── types/
│       └── index.ts                  # TypeScript type definitions
│
└── [config files]                    # TypeScript, ESLint, Vitest, Tailwind, etc.
```

### Key Files

- **`src/lib/api.ts`**: All backend API calls, typed with Axios
- **`src/lib/schemas.ts`**: Zod schemas for form validation and for parsing API responses (the backend stores goals with domain field names such as `target_amount`; the response schemas map them to the form shape)
- **`src/features/goals/components/DayStrip.tsx`**: The 14-day strip and its click-to-act popover
- **`src/lib/constants.ts`**: Statistic type metadata, icons, and descriptions
- **`src/features/goals/components/GoalForm.tsx`**: Polymorphic form that renders different fields per statistic type
- **`src/features/goals/components/fields/`**: Type-specific goal input components

## Integration with LifeStats Backend

### API Configuration

The web UI connects to the LifeStats Rust backend via REST API:

**Default API Base URL**: `http://localhost:3000`

**Override via environment variable**:
```bash
# .env.local
NEXT_PUBLIC_API_URL=http://localhost:3000
```

### Backend Endpoints Used

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/statistics` | GET | Fetch all statistics |
| `/statistics/loggable` | GET | Fetch only loggable statistics |
| `/statistics` | POST | Create new statistic |
| `/statistics/:id/achievements` | GET | Per-day achievement + override state for the day strip |
| `/statistics/:id/goals/default` | GET | Get default goal |
| `/statistics/:id/goals/default` | PUT | Set default goal |
| `/statistics/:id/goals/default` | DELETE | Delete default goal |
| `/statistics/:id/goals/:date` | GET | Daily override state (`not_set` / `suppressed` / `goal`) |
| `/statistics/:id/goals/:date` | PUT | Set daily goal override |
| `/statistics/:id/goals/:date` | DELETE | Delete daily goal override |

### Type Safety & Data Serialization

The web UI maintains strong type safety between frontend and backend:

1. **TypeScript Types** (`src/types/index.ts`): Mirror backend DTOs
2. **Zod Schemas** (`src/lib/schemas.ts`): Runtime validation at API boundary
3. **Goal Data Serialization**:
   - **Backend expects**: `Record<string, string>` (string maps)
   - **Frontend uses**: Typed objects (e.g., `{ target: number, is_at_least: boolean }`)
   - **Backend returns**: Domain field names (e.g., `{ target_amount: 100, is_at_least: true }`)
   - **Conversion functions**:
     - `serializeGoalData()`: Typed data → string map (for PUT requests)
     - `parseGoalData()`: Response object → typed data with Zod validation (for GET responses and the `default_goal` carried by `GET /statistics`)

## Statistic Types

The web UI supports all 6 statistic types from the LifeStats domain model:

### Loggable Statistics (Accept Direct Input)
1. **IntegerAmount**: Numeric counts (e.g., pages read, calories consumed)
2. **Habit**: Boolean occurrences (e.g., exercised today, meditated)
3. **PointInTime**: Specific timestamps (e.g., wake time, bedtime)
4. **TimeSpans**: Time intervals with start/end (e.g., work sessions, sleep periods)

### Derived Statistics (Computed from Others)
5. **Summing**: Aggregates multiple TimeSpans statistics
6. **TimeDifference**: Delta between two PointInTime statistics

**Note**: Derived statistics require source statistics to be specified during creation. The form validates these requirements.

## Development Workflow

### Available Scripts

```bash
npm run dev          # Start development server (localhost:3001)
npm run build        # Production build
npm start            # Start production server
npm run lint         # Run ESLint
npm run type-check   # TypeScript type checking (no emit)
npm test             # Vitest unit tests (pure helpers: formatting, grouping, parsing, day status)
```

### Code Quality Tools

**Pre-commit Hooks** (via Husky + lint-staged):
- `.husky/pre-commit` runs `lint-staged` (ESLint with auto-fix + `tsc-files` on staged files)
- Only active when git's `core.hooksPath` points at `ui/web-ui/.husky`; the repo does not enable it by default

**Unit tests** (Vitest, `*.test.ts` next to the code): cover `goalFormat`, `statisticGroups`, `schemas` (real backend response shapes) and `dayStatus`. Components are verified manually against the running backend.

**ESLint Configuration**:
- Extends Next.js core web vitals + TypeScript rules
- Run manually: `npm run lint`

**TypeScript**:
- Strict mode enabled
- Path alias: `@/*` → `./src/*`
- Type-check without emit: `npm run type-check`

### Key Development Patterns

#### 1. Server State Management (React Query)

```typescript
// features/statistics/hooks/useStatistics.ts
const { data: statistics, isLoading, error } = useStatistics();
```

React Query handles:
- Automatic caching
- Background refetching
- Cache invalidation after mutations
- Loading/error states

#### 2. Form State Management (React Hook Form + Zod)

```typescript
const form = useForm<FormData>({
  resolver: zodResolver(schema),
  defaultValues: { /* ... */ }
});
```

Benefits:
- Type-safe form data
- Runtime validation with Zod schemas
- Automatic error messages
- Controlled/uncontrolled input flexibility

#### 3. Polymorphic Components

The `GoalForm` component adapts to different statistic types:

```typescript
// Renders different fields based on statistic.type
{statistic.type === 'IntegerAmount' && <IntegerAmountFields />}
{statistic.type === 'Habit' && <HabitFields />}
{statistic.type === 'TimeSpans' && <TimeSpansFields />}
// etc.
```

Each field component is type-safe and validates its specific goal data structure.

## Architecture Highlights

### Feature-Based Organization

Components are organized by feature domain (`statistics`, `goals`) rather than technical role. This keeps related code together and makes features easier to understand and modify.

### Separation of Concerns

- **API Layer** (`lib/api.ts`): Handles HTTP communication
- **Hooks** (`features/*/hooks/`): Encapsulate data fetching and mutations
- **Components** (`features/*/components/`): Pure presentation logic
- **Schemas** (`lib/schemas.ts`): Validation boundary

### Type Safety at API Boundary

All API responses are validated with Zod schemas before being used in the application. This catches runtime mismatches between frontend expectations and backend reality.

```typescript
// Parse and validate API response
const parsed = statisticSchema.parse(response.data);
```

## Known Limitations & Future Work

### Current Limitations

1. **TimeDifference Goal Editing**: Not yet implemented (UI shows warning message)
2. **No Authentication**: All endpoints are publicly accessible
3. **No Data Logging UI**: Must use logger applications or API directly
4. **No Plot Visualization**: Beyond the 14-day strip, plot endpoints have no UI integration

### Potential Future Enhancements

- Add data logging interface (quick log form on dashboard)
- Integrate plot visualization (show charts in statistic cards)
- Add authentication/authorization
- Add statistics filtering and search
- Add bulk goal operations (e.g. suppress a date range across all goals)
- Add goal history view (track changes over time)
- Add keyboard shortcuts for power users
- Add dark mode support

## Troubleshooting

### "Failed to fetch statistics"

**Cause**: Backend is not running or not accessible

**Solution**:
1. Start the backend: `cargo run -p lifestats-web-api` (from project root)
2. Verify backend is on `http://localhost:3000`
3. Check `NEXT_PUBLIC_API_URL` if using custom port

### "Port 3001 already in use"

**Cause**: The production UI service (`lifestats-web-ui.service`) is running

**Solution**: Run the dev server on another port:
```bash
npm run dev -- -p 3002
```

### Type errors on `npm run dev`

**Cause**: Dependencies out of sync or TypeScript version mismatch

**Solution**:
```bash
rm -rf node_modules package-lock.json
npm install
```

### Pre-commit hook failing

**Cause**: ESLint errors or TypeScript type errors

**Solution**:
```bash
npm run lint -- --fix  # Auto-fix ESLint issues
npm run type-check     # See type errors
```

## Contributing

When adding new features to the web UI:

1. **Follow feature-based structure**: Add components to appropriate `features/` subdirectory
2. **Use TypeScript strictly**: No `any` types, prefer type inference
3. **Validate at API boundary**: Add Zod schemas for new API responses
4. **Use React Query for server state**: Don't store server data in local state
5. **Test type safety**: Run `npm run type-check` before committing
6. **Update this README**: Document new features, routes, or configuration

## Related Documentation

- **Project Overview**: `../../CLAUDE.md` - LifeStats architecture and domain model
- **Backend API**: `../../core/infrastructure/web_api/README.md` - API endpoint documentation
- **Architecture**: `../../docs/ARCHITECTURE.md` - Hexagonal architecture details
- **Statistics Guide**: `../../docs/statistics.md` - Statistic type behaviors

## License

Part of the LifeStats project. See project root for license information.
