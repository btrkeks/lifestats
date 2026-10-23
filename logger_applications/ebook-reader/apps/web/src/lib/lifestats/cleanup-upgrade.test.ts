/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

// @vitest-environment jsdom
import 'fake-indexeddb/auto';

import { startLifestatsSync } from '@lifestats/ttu-sync';
import { deleteDB, openDB } from 'idb';
import { afterEach, expect, it, vi } from 'vitest';

import { createBooksDb } from '../data/database/books-db/factory';

const book = {
  id: 1,
  title: '影響力の武器',
  elementHtml: '<p>影響力の武器</p>',
  characters: 6,
  blobs: {},
  sections: [{ reference: 'ttu-0', charactersWeight: 6 }],
  lastBookModified: 1_700_000_000_000
};
const bookmark = { dataId: 1, exploredCharCount: 4, scrollX: 20, scrollY: 0, progress: '66.7' };
const day = {
  book: `ttu-text-v1:${'a'.repeat(64)}`,
  day: '2026-10-22',
  title: book.title,
  length: 6,
  spans: [[0, 4]],
  lastPosition: 4,
  observedAt: Date.parse('2026-10-22T12:00:00Z'),
  revision: 3,
  delivered: { revision: 2, serverId: '5b0c1f8e-2f4d-4c43-9a43-7c1fd0b5a001' },
  delivery: { kind: 'ready' }
};

afterEach(async () => {
  await Promise.all(
    ['books', 'ttu-lifestats', 'ttu-lifestats-legacy'].map((name) => deleteDB(name))
  );
  vi.unstubAllGlobals();
});

it('upgrades a v8 profile to books v9 and sync v2 without changing books, bookmarks or days', async () => {
  vi.stubGlobal('BroadcastChannel', undefined);
  const books = await openDB('books', 8, {
    upgrade(db) {
      db.createObjectStore('data', { keyPath: 'id' });
      db.createObjectStore('bookmark', { keyPath: 'dataId' });
      db.createObjectStore('ebookSyncState', { keyPath: 'dataId' });
      db.createObjectStore('ebookSyncQueue', { keyPath: 'eventId' });
    }
  });
  await books.put('data', book);
  await books.put('bookmark', bookmark);
  await books.put('ebookSyncState', { dataId: 1, confirmedPosition: 4, lastFlushedPosition: 4 });
  const dataBefore = JSON.stringify(await books.getAll('data'));
  const bookmarkBefore = JSON.stringify(await books.getAll('bookmark'));
  books.close();

  const previous = await openDB('ttu-lifestats', 1, {
    upgrade(db) {
      db.createObjectStore('meta');
      db.createObjectStore('identities', { keyPath: 'localId' });
      db.createObjectStore('days', { keyPath: ['book', 'day'] });
      db.createObjectStore('claims', { keyPath: 'readerDataId' });
    }
  });
  await previous.put('meta', { producerId: '0b6c3f0e-2a71-4d38-9f43-5a1e8c2d9b77' }, 'producer');
  await previous.put('identities', {
    localId: 1,
    modified: book.lastBookModified,
    key: day.book,
    length: 6
  });
  const metaBefore = JSON.stringify(await previous.getAll('meta'));
  const identitiesBefore = JSON.stringify(await previous.getAll('identities'));
  await previous.put('days', day);
  await previous.put('claims', { readerDataId: 1, book: day.book });
  const daysBefore = JSON.stringify(await previous.getAll('days'));
  previous.close();
  const receipts = await openDB('ttu-lifestats-legacy', 1, {
    upgrade(db) {
      db.createObjectStore('receipts', { keyPath: 'id' });
    }
  });
  await receipts.put('receipts', { id: 'claim:1', sentTo: day.delivered.serverId });
  receipts.close();

  const upgraded = await createBooksDb();
  const sync = startLifestatsSync({ buildId: 'cleanup-test' });
  await new Promise<void>((resolve) => {
    const stop = sync.status.subscribe((status) => {
      if (status.kind !== 'starting') {
        queueMicrotask(stop);
        resolve();
      }
    });
  });
  expect(upgraded.version).toBe(9);
  expect([...upgraded.objectStoreNames]).not.toContain('ebookSyncState');
  expect([...upgraded.objectStoreNames]).not.toContain('ebookSyncQueue');
  expect(JSON.stringify(await upgraded.getAll('data'))).toBe(dataBefore);
  expect(JSON.stringify(await upgraded.getAll('bookmark'))).toBe(bookmarkBefore);
  upgraded.close();
  const current = await openDB('ttu-lifestats');
  expect(current.version).toBe(2);
  expect(current.objectStoreNames.contains('claims')).toBe(false);
  expect(JSON.stringify(await current.getAll('days'))).toBe(daysBefore);
  expect(JSON.stringify(await current.getAll('meta'))).toBe(metaBefore);
  expect(JSON.stringify(await current.getAll('identities'))).toBe(identitiesBefore);
  current.close();
  expect((await indexedDB.databases()).some(({ name }) => name === 'ttu-lifestats-legacy')).toBe(
    false
  );
});
