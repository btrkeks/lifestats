# Ebook Progress Web UI Handoff

## Current State

The ebook reader now syncs two separate things when reading progress advances:

- A classic daily statistic, `[Ebook] Characters Read`, through the existing ebook event ingestion path.
- A newer `ProgressItem` for the current book, with absolute character-position checkpoints.

The existing web UI dashboard at `ui/web-ui/src/app/dashboard/page.tsx` only calls `GET /statistics` through `useStatistics()`. It will not show ebook book progress until the web UI is taught to fetch and render `ProgressItem`s.

## Backend API To Use

The Rust API base URL is configured in `ui/web-ui/src/lib/api.ts` via `NEXT_PUBLIC_API_URL`, defaulting to `http://localhost:3000`.

Use these endpoints:

- `GET /progress-items`
  Returns all progress items. The HTTP route does not currently expose tag filtering, so filter ebook items client-side for now.

- `GET /progress-items/{id}/summary`
  Returns the item plus latest/furthest checkpoint and completion percentages.

- Optional: `GET /progress-items/{id}/daily-deltas?start=YYYY-MM-DD&end=YYYY-MM-DD`
  Useful later for charts, but not required for a first tab.

Relevant response shapes:

```ts
type ProgressUnit =
  | { type: 'Pages' }
  | { type: 'Characters' }
  | { type: 'Percent' }
  | { type: 'Chapters' }
  | { type: 'Custom'; label: string };

interface ProgressItem {
  id: number;
  name: string;
  category: string;
  target_value: number;
  unit: ProgressUnit;
  tags: Record<string, string>;
  created_at: string;
  updated_at: string;
}

interface ProgressCheckpoint {
  id: number | null;
  item_id: number;
  recorded_at: string;
  effective_date: string;
  position: number;
  source: string | null;
  source_event_id: string | null;
  note: string | null;
  created_at: string;
}

interface ProgressSummary {
  item: ProgressItem;
  latest_checkpoint: ProgressCheckpoint | null;
  latest_position: number | null;
  furthest_position: number | null;
  percent_latest: number | null;
  percent_furthest: number | null;
  is_complete: boolean;
}
```

Ebook book progress items are identified by tags:

```ts
item.tags.source === 'ebook'
item.tags.progress_type === 'book'
item.tags.reader_data_id // ebook reader local book id
item.tags.book_title // human-readable title captured when provisioned
```

## Suggested UI Implementation

Add a new tab on the dashboard, for example `Statistics | Book Progress`.

For the first version:

1. Add `ProgressItem`, `ProgressSummary`, and related types to `ui/web-ui/src/types/index.ts`.
2. Add API helpers in `ui/web-ui/src/lib/api.ts`:
   - `getProgressItems(): Promise<ProgressItem[]>`
   - `getProgressSummary(id: number): Promise<ProgressSummary>`
3. Add a React Query hook, likely under a new `features/progress/` folder:
   - Fetch `GET /progress-items`.
   - Filter to ebook book items using the tags above.
   - Fetch summaries for the filtered item ids.
4. Render a compact table/card list:
   - Book title: prefer `summary.item.name`; fall back to `tags.book_title`.
   - Progress: use `percent_furthest ?? percent_latest`, formatted as a percentage.
   - Position: `furthest_position ?? latest_position ?? 0` out of `target_value`.
   - Unit label: `Characters` for ebook items.
   - Last updated: `latest_checkpoint.recorded_at` if present, otherwise `item.updated_at`.
5. Empty state should say something like: “No ebook progress has been synced yet. Open the reader and make new forward progress.”

Avoid mixing these items into the existing classic `StatisticGrid`: `ProgressItem`s are not `StatisticMetadata`, do not have goals, and are backed by `/progress-items`, not `/statistics`.

## Behavior Notes For Debugging

The reader only sends an event after confirmed forward progress beyond its stored baseline. Opening a book and jumping to the current bookmark may not create a new progress item immediately. New forward movement, or waiting through suspicious-jump confirmation, may be needed.

To verify backend data independently of the web UI:

```bash
curl http://localhost:3000/progress-items
curl http://localhost:3000/progress-items/<id>/summary
```

If the web UI runs on `localhost:3001`, that is only the Next.js frontend. It still talks to the Rust API via `NEXT_PUBLIC_API_URL`, usually `http://localhost:3000`.
