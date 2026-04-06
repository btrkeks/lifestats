import 'fake-indexeddb/auto';

import { afterEach, describe, expect, it } from 'vitest';
import { deleteDB } from 'idb';

import { deleteEbookSyncRecordsForDataId } from './ebook-sync-cleanup';
import { createBooksDb } from './factory';

const createdDbNames = new Set<string>();

function dbName(prefix: string) {
  const name = `${prefix}-${crypto.randomUUID()}`;
  createdDbNames.add(name);
  return name;
}

afterEach(async () => {
  await Promise.all([...createdDbNames].map((name) => deleteDB(name)));
  createdDbNames.clear();
});

describe('deleteEbookSyncRecordsForDataId', () => {
  it('removes sync state and queued events for a deleted book when local statistics are also deleted', async () => {
    const db = await createBooksDb(dbName('books-sync-cleanup-delete-all'));

    try {
      await db.put('ebookSyncState', {
        dataId: 1,
        currentPosition: 120,
        confirmedPosition: 120,
        lastFlushedPosition: 80,
        bundleVersion: 1,
        updatedAt: 1
      });
      await db.put('ebookSyncQueue', {
        eventId: 'queue-a',
        dataId: 1,
        dateKey: '2026-04-05',
        metric: 'characters_read',
        delta: 40,
        confirmedPositionAfter: 120,
        bookTitle: 'Book A',
        createdAt: 1,
        attemptCount: 0,
        nextRetryAt: 0
      });
      await db.put('ebookSyncQueue', {
        eventId: 'queue-other',
        dataId: 2,
        dateKey: '2026-04-05',
        metric: 'characters_read',
        delta: 10,
        confirmedPositionAfter: 10,
        bookTitle: 'Other',
        createdAt: 2,
        attemptCount: 0,
        nextRetryAt: 0
      });

      const tx = db.transaction(['ebookSyncState', 'ebookSyncQueue'], 'readwrite');
      await deleteEbookSyncRecordsForDataId(tx, 1, true);
      await tx.done;

      expect(await db.get('ebookSyncState', 1)).toBeUndefined();
      expect(await db.get('ebookSyncQueue', 'queue-a')).toBeUndefined();
      expect(await db.get('ebookSyncQueue', 'queue-other')).toBeDefined();
    } finally {
      db.close();
    }
  });

  it('removes sync state but keeps queued events when local statistics are retained', async () => {
    const db = await createBooksDb(dbName('books-sync-cleanup-keep-queue'));

    try {
      await db.put('ebookSyncState', {
        dataId: 1,
        currentPosition: 90,
        confirmedPosition: 90,
        lastFlushedPosition: 50,
        bundleVersion: 1,
        updatedAt: 1
      });
      await db.put('ebookSyncQueue', {
        eventId: 'queue-b',
        dataId: 1,
        dateKey: '2026-04-05',
        metric: 'characters_read',
        delta: 40,
        confirmedPositionAfter: 90,
        bookTitle: 'Book B',
        createdAt: 1,
        attemptCount: 0,
        nextRetryAt: 0
      });

      const tx = db.transaction(['ebookSyncState', 'ebookSyncQueue'], 'readwrite');
      await deleteEbookSyncRecordsForDataId(tx, 1, false);
      await tx.done;

      expect(await db.get('ebookSyncState', 1)).toBeUndefined();
      expect(await db.get('ebookSyncQueue', 'queue-b')).toBeDefined();
    } finally {
      db.close();
    }
  });
});
