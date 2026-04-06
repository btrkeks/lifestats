# Ebook Reader Character Sync Review

Date: 2026-04-05

Scope: review of the `ebook-reader-character-sync-adr` implementation across the ebook-reader web app and the Rust integration/persistence/API path.

Review method:
- One focused backend review pass
- One focused frontend review pass
- One local reconciliation pass across the edited files

## Findings

### 1. High: IndexedDB upgrade path does not create the new ebook sync stores for most databases

File:
- `logger_applications/ebook-reader/apps/web/src/lib/data/database/books-db/factory.ts`

Problem:
- The new `ebookSyncState` and `ebookSyncQueue` stores are only created in `case 6`.
- Earlier cases terminate with `break`, so fresh databases (`oldVersion === 0`) and upgrades from versions `2` through `5` never reach `case 6`.
- That means the database can open as version 7 without the required stores.

Impact:
- The first call to the new sync APIs in `database.service.ts` can fail at runtime with missing object store errors.
- This is a real correctness issue for new installs and older existing readers.

Related code:
- `logger_applications/ebook-reader/apps/web/src/lib/data/database/books-db/factory.ts`
- `logger_applications/ebook-reader/apps/web/src/lib/data/database/books-db/database.service.ts`

### 2. Medium: Book deletion leaves orphaned ebook sync state and queued outbound events

File:
- `logger_applications/ebook-reader/apps/web/src/lib/data/database/books-db/database.service.ts`

Problem:
- The new ebook sync records are not removed when a book is deleted.
- That includes both persisted `ebookSyncState` and queued entries in `ebookSyncQueue`.

Impact:
- Deleted books can still have pending `characters_read` events flushed later.
- This creates stale local state and can produce incorrect backend logging after deletion.

Related code:
- `logger_applications/ebook-reader/apps/web/src/lib/data/database/books-db/database.service.ts`

## Backend Review Result

No material correctness findings were identified in the Rust implementation.

Reviewed areas:
- `core/application/src/use_cases/integration/*`
- `core/infrastructure/persistence/*`
- `core/infrastructure/web_api/src/routes/*`
- `core/infrastructure/web_api/tests/*`
- `lifestats-bootstrap/src/lib.rs`

The route contract, idempotent event store behavior, and targeted Rust tests looked coherent.

## Residual Risks

### Frontend migration coverage

There is currently no targeted test coverage proving:
- fresh DB creation includes the new sync stores
- older DB versions upgrade correctly into the v7 schema

### Frontend state-machine coverage

The new reading progress confirmation logic would benefit from focused tests around:
- suspicious forward jumps
- dwell-based confirmation
- continued-forward confirmation
- backward movement
- pause/freeze/unfreeze behavior

Relevant file:
- `logger_applications/ebook-reader/apps/web/src/lib/data/lifestats/reading-progress-tracker.ts`

### End-to-end duplicate ingestion coverage

Persistence-level idempotency is tested, but there is not yet a real end-to-end HTTP test that submits the same ebook event twice against the real route and persistence stack and verifies the duplicate behavior.
