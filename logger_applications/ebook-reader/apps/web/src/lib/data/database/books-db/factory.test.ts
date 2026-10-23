/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import 'fake-indexeddb/auto';

import { afterEach, describe, expect, it } from 'vitest';
import { deleteDB, openDB, type IDBPDatabase } from 'idb';
import { firstValueFrom } from 'rxjs';
import { filter } from 'rxjs/operators';

import { BOOKS_DB_VERSION, booksDbUpgradeBlocked$, createBooksDb } from './factory';

const upstreamStores = [
  'audioBook',
  'bookmark',
  'data',
  'handle',
  'lastItem',
  'lastModified',
  'readingGoal',
  'statistic',
  'storageSource',
  'subtitle'
];

const book = {
  id: 1,
  title: '吾輩は猫である',
  styleSheet: '.a{color:red}',
  elementHtml: '<div id="ttu-0"><p>吾輩は<ruby>猫<rt>ねこ</rt></ruby>である。</p></div>',
  blobs: {},
  coverImage: 'ttu:cover',
  hasThumb: false,
  characters: 8,
  sections: [{ reference: 'ttu-0', charactersWeight: 8 }],
  lastBookModified: 1_700_000_000_000,
  lastBookOpen: 1_700_000_100_000
};
const bookmark = {
  dataId: 1,
  scrollX: 0,
  scrollY: 120,
  exploredCharCount: 5,
  progress: '62.5',
  lastBookmarkModified: 1_700_000_200_000
};
const syncState = {
  dataId: 1,
  currentPosition: 5,
  confirmedPosition: 5,
  lastFlushedPosition: 3,
  bundleVersion: 1,
  updatedAt: 1_700_000_300_000
};
const queuedEvent = {
  eventId: '0f8fad5b-d9cb-469f-a165-70867728950e',
  dataId: 1,
  dateKey: '2023-11-14',
  metric: 'characters_read',
  delta: 3,
  confirmedPositionAfter: 3,
  bookTargetCharacters: 8,
  bookTitle: '吾輩は猫である',
  createdAt: 1_700_000_250_000,
  attemptCount: 2,
  nextRetryAt: 1_700_000_260_000,
  lastError: 'Failed to fetch'
};

const names: string[] = [];

function dbName() {
  const name = `books-${crypto.randomUUID()}`;
  names.push(name);
  return name;
}

afterEach(async () => {
  await Promise.all(names.splice(0).map((name) => deleteDB(name)));
});

async function seed(name: string, version: number, upgrade: (db: IDBPDatabase) => void) {
  const db = await openDB(name, version, { upgrade });
  await db.put('data', book);
  await db.put('bookmark', bookmark);
  return db;
}

async function contents(name: string) {
  const db = await openDB(name);
  try {
    const stores = [...db.objectStoreNames];
    const rows = Object.fromEntries(
      await Promise.all(stores.map(async (store) => [store, await db.getAll(store)] as const))
    );
    return { version: db.version, stores, rows };
  } finally {
    db.close();
  }
}

describe('createBooksDb', () => {
  it('creates the upstream schema without Lifestats stores', async () => {
    const name = dbName();
    (await createBooksDb(name)).close();

    const { version, stores } = await contents(name);
    expect(version).toBe(BOOKS_DB_VERSION);
    expect(stores).toEqual(upstreamStores);
  });

  it.each([7, 8])(
    'opens a v%i database at v9, drops retired stores and keeps upstream rows',
    async (oldVersion) => {
      const name = dbName();
      const legacy = await seed(name, oldVersion, (db) => {
        const data = db.createObjectStore('data', { keyPath: 'id', autoIncrement: true });
        data.createIndex('title', 'title');
        db.createObjectStore('bookmark', { keyPath: 'dataId' });
        db.createObjectStore('lastItem');
        db.createObjectStore('storageSource', { keyPath: 'name' });
        db.createObjectStore('statistic', { keyPath: ['title', 'dateKey'] });
        db.createObjectStore('readingGoal', { keyPath: 'goalStartDate' });
        db.createObjectStore('lastModified', { keyPath: ['title', 'dataType'] });
        db.createObjectStore('audioBook', { keyPath: 'title' });
        db.createObjectStore('subtitle', { keyPath: 'title' });
        db.createObjectStore('handle', { keyPath: ['title', 'dataType'] });
        db.createObjectStore('ebookSyncState', { keyPath: 'dataId' });
        const queue = db.createObjectStore('ebookSyncQueue', { keyPath: 'eventId' });
        queue.createIndex('nextRetryAt', 'nextRetryAt');
        queue.createIndex('dataId', 'dataId');
      });
      await legacy.put('ebookSyncState', syncState);
      await legacy.put('ebookSyncQueue', queuedEvent);
      legacy.close();
      const before = await contents(name);

      (await createBooksDb(name)).close();

      const after = await contents(name);
      expect(after.version).toBe(9);
      expect(after.stores).toEqual(upstreamStores);
      const { ebookSyncQueue: _queue, ebookSyncState: _state, ...upstreamRows } = before.rows;
      expect(JSON.stringify(after.rows)).toBe(JSON.stringify(upstreamRows));
    }
  );

  it('upgrades a 2023 v4 database to every upstream store and keeps its book and bookmark', async () => {
    const name = dbName();
    const legacy = await seed(name, 4, (db) => {
      const data = db.createObjectStore('data', { keyPath: 'id', autoIncrement: true });
      data.createIndex('title', 'title');
      db.createObjectStore('bookmark', { keyPath: 'dataId' });
      db.createObjectStore('lastItem');
      db.createObjectStore('storageSource', { keyPath: 'name' });
    });
    await legacy.put('lastItem', { dataId: 1 }, 0);
    legacy.close();

    (await createBooksDb(name)).close();

    const after = await contents(name);
    expect(after.stores).toEqual(upstreamStores);
    expect(after.rows.data).toEqual([book]);
    expect(after.rows.bookmark).toEqual([bookmark]);
    expect(after.rows.lastItem).toEqual([{ dataId: 1 }]);
  });

  it('reports a blocked upgrade while an old tab holds v7 open, then completes', async () => {
    const name = dbName();
    const oldTab = await openDB(name, 7, {
      upgrade(db) {
        db.createObjectStore('data', { keyPath: 'id', autoIncrement: true });
      }
    });

    const opening = createBooksDb(name);
    await firstValueFrom(booksDbUpgradeBlocked$.pipe(filter(Boolean)));
    oldTab.close();
    (await opening).close();

    expect(booksDbUpgradeBlocked$.getValue()).toBe(false);
  });

  it('closes its own connection when a newer version is requested', async () => {
    const name = dbName();
    await createBooksDb(name);

    const newer = await openDB(name, BOOKS_DB_VERSION + 1);

    expect(newer.version).toBe(BOOKS_DB_VERSION + 1);
    newer.close();
  });
});
