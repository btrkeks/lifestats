# LifeStats Web UI

A modern Next.js web application for managing statistics and goals in the LifeStats personal observability platform.

## Overview

The Web UI is the **goal management interface** for LifeStats. It provides a clean, type-safe web interface for creating statistics and managing their goals (both default goals and daily overrides).

**Tech Stack:**
- **Framework**: Next.js 15.5.4 (App Router)
- **Language**: TypeScript 5
- **UI**: React 19.1.0 + Tailwind CSS 4
- **State Management**: TanStack Query (React Query) v5
- **Form Handling**: React Hook Form v7 + Zod validation
- **HTTP Client**: Axios v1.12.2

## Features

### What This UI Provides

- **Statistics Dashboard** (`/dashboard`): Browse all statistics with quick access to goal management
- **Statistic Creation** (`/statistics/new`): Create new statistics with type-specific configuration
- **Goal Management**: Set and edit default goals and daily goal overrides for any statistic
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

This starts the development server on `http://localhost:3000` (or the next available port if 3000 is in use).

### ⚠️ Port Conflict Warning

By default, both the web-ui dev server and the LifeStats backend bind to port 3000. You have two options:

1. **Run web-ui on a different port** (recommended):
   ```bash
   npm run dev -- -p 3001
   ```

2. **Configure API URL** if backend is on a different port:
   ```bash
   # Create .env.local
   echo "NEXT_PUBLIC_API_URL=http://localhost:3000" > .env.local
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
│   │   ├── page.tsx                  # Landing page (/)
│   │   ├── layout.tsx                # Root layout with providers
│   │   ├── dashboard/                # Statistics dashboard
│   │   └── statistics/new/           # Statistic creation flow
│   │
│   ├── features/                     # Feature-based components
│   │   ├── statistics/               # Statistic management
│   │   │   ├── components/           # StatisticCard, StatisticGrid, etc.
│   │   │   └── hooks/                # useStatistics hook
│   │   └── goals/                    # Goal management
│   │       ├── components/           # Goal forms and editors
│   │       │   └── fields/           # Type-specific goal input fields
│   │       └── hooks/                # useGoals hook
│   │
│   ├── lib/                          # Core utilities
│   │   ├── api.ts                    # API client (axios wrapper)
│   │   ├── constants.ts              # Statistic type definitions & icons
│   │   ├── providers.tsx             # React Query provider setup
│   │   └── schemas.ts                # Zod schemas for validation
│   │
│   └── types/
│       └── index.ts                  # TypeScript type definitions
│
├── public/                           # Static assets
└── [config files]                    # TypeScript, ESLint, Tailwind, etc.
```

### Key Files

- **`src/lib/api.ts`**: All backend API calls, typed with Axios
- **`src/lib/schemas.ts`**: Zod schemas for runtime validation of API responses
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
| `/statistics/:id` | GET | Fetch single statistic |
| `/statistics` | POST | Create new statistic |
| `/statistics/:id` | DELETE | Delete statistic |
| `/statistics/:id/goals/default` | GET | Get default goal |
| `/statistics/:id/goals/default` | PUT | Set default goal |
| `/statistics/:id/goals/default` | DELETE | Delete default goal |
| `/statistics/:id/goals/:date` | GET | Get daily goal for specific date |
| `/statistics/:id/goals/:date` | PUT | Set daily goal override |
| `/statistics/:id/goals/:date` | DELETE | Delete daily goal override |

### Type Safety & Data Serialization

The web UI maintains strong type safety between frontend and backend:

1. **TypeScript Types** (`src/types/index.ts`): Mirror backend DTOs
2. **Zod Schemas** (`src/lib/schemas.ts`): Runtime validation at API boundary
3. **Goal Data Serialization**:
   - **Backend expects**: `Record<string, string>` (string maps)
   - **Frontend uses**: Typed objects (e.g., `{ target: number, isHigherBetter: boolean }`)
   - **Conversion functions**:
     - `serializeGoalData()`: Typed data → string map (for PUT requests)
     - `parseGoalData()`: String map → typed data with Zod validation (for GET responses)

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
npm run dev          # Start development server (default: localhost:3000)
npm run build        # Production build
npm start            # Start production server
npm run lint         # Run ESLint
npm run type-check   # TypeScript type checking (no emit)
```

### Code Quality Tools

**Pre-commit Hooks** (via Husky + lint-staged):
- Automatically runs on `git commit`
- ESLint with auto-fix on staged files
- TypeScript type checking on staged files
- Prevents commits with type errors or lint issues

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
4. **No Plot Visualization**: Plot endpoints exist in backend but no UI integration yet
5. **Port Conflict**: Default port 3000 conflicts with backend (use `-p 3001` flag)

### Potential Future Enhancements

- Add data logging interface (quick log form on dashboard)
- Integrate plot visualization (show charts in statistic cards)
- Add authentication/authorization
- Add statistics filtering and search
- Add bulk goal operations
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

### "Port 3000 already in use"

**Cause**: Backend is using port 3000

**Solution**: Run web-ui on different port:
```bash
npm run dev -- -p 3001
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
