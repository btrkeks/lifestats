import { afterEach, describe, expect, it, vi } from 'vitest';

import type { DatabaseService } from '$lib/data/database/books-db/database.service';
import type { BooksDbEbookSyncQueueEntry } from '$lib/data/database/books-db/versions/books-db';

import { LifestatsSyncAdapter } from './lifestats-sync-adapter';

function database(entries: BooksDbEbookSyncQueueEntry[]) {
  return {
    getEbookSyncQueueEntries: vi.fn().mockResolvedValue(entries),
    deleteEbookSyncQueueEntry: vi.fn().mockResolvedValue(undefined),
    putEbookSyncQueueEntry: vi.fn().mockResolvedValue(undefined)
  } as unknown as DatabaseService;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('LifestatsSyncAdapter', () => {
  it('stores book target characters on new queue entries', () => {
    const adapter = new LifestatsSyncAdapter(database([]));

    const entry = adapter.createQueueEntry(7, '2026-05-02', 42, 420, 1_000, 'Book');

    expect(entry.bookTargetCharacters).toBe(1_000);
  });

  it('sends book target characters when flushing queue entries', async () => {
    const entry: BooksDbEbookSyncQueueEntry = {
      eventId: 'evt-1',
      dataId: 7,
      dateKey: '2026-05-02',
      metric: 'characters_read',
      delta: 42,
      confirmedPositionAfter: 420,
      bookTargetCharacters: 1_000,
      bookTitle: 'Book',
      createdAt: 1,
      attemptCount: 0,
      nextRetryAt: 0
    };
    const db = database([entry]);
    const fetchMock = vi.fn().mockResolvedValue({ ok: true });
    vi.stubGlobal('fetch', fetchMock);

    const adapter = new LifestatsSyncAdapter(db, 'http://localhost:3000');
    await adapter.flushQueue(true);

    const eventRequest = fetchMock.mock.calls.find(([url]) =>
      String(url).endsWith('/integrations/ebook/events')
    );
    expect(eventRequest).toBeDefined();

    const payload = JSON.parse(eventRequest?.[1]?.body as string);
    expect(payload.book_target_characters).toBe(1_000);
  });

  it('falls back to confirmed position for legacy queued entries without a target', async () => {
    const entry: BooksDbEbookSyncQueueEntry = {
      eventId: 'evt-1',
      dataId: 7,
      dateKey: '2026-05-02',
      metric: 'characters_read',
      delta: 42,
      confirmedPositionAfter: 420,
      bookTitle: 'Book',
      createdAt: 1,
      attemptCount: 0,
      nextRetryAt: 0
    };
    const db = database([entry]);
    const fetchMock = vi.fn().mockResolvedValue({ ok: true });
    vi.stubGlobal('fetch', fetchMock);

    const adapter = new LifestatsSyncAdapter(db, 'http://localhost:3000');
    await adapter.flushQueue(true);

    const eventRequest = fetchMock.mock.calls.find(([url]) =>
      String(url).endsWith('/integrations/ebook/events')
    );
    const payload = JSON.parse(eventRequest?.[1]?.body as string);

    expect(payload.book_target_characters).toBe(420);
  });
});
