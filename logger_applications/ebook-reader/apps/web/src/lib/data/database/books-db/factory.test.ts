import 'fake-indexeddb/auto';

import { afterEach, describe, expect, it } from 'vitest';
import { deleteDB, openDB } from 'idb';

import { createBooksDb } from './factory';

const expectedModernStores = [
  'data',
  'bookmark',
  'lastItem',
  'storageSource',
  'statistic',
  'readingGoal',
  'lastModified',
  'audioBook',
  'subtitle',
  'handle',
  'ebookSyncState',
  'ebookSyncQueue'
];

const createdDbNames = new Set<string>();

function dbName(prefix: string) {
  const name = `${prefix}-${crypto.randomUUID()}`;
  createdDbNames.add(name);
  return name;
}

async function closeAndDelete(name: string) {
  await deleteDB(name);
}

afterEach(async () => {
  await Promise.all([...createdDbNames].map((name) => closeAndDelete(name)));
  createdDbNames.clear();
});

describe('createBooksDb', () => {
  it('creates the ebook sync stores for a fresh v7 database', async () => {
    const db = await createBooksDb(dbName('books-fresh'));

    try {
      expect([...db.objectStoreNames]).toEqual(expect.arrayContaining(expectedModernStores));
    } finally {
      db.close();
    }
  });

  it('upgrades a v3 database cumulatively to the full v7 schema', async () => {
    const name = dbName('books-v3-upgrade');
    const legacyDb = await openDB(name, 3, {
      upgrade(db) {
        const dataStore = db.createObjectStore('data', {
          keyPath: 'id',
          autoIncrement: true
        });
        dataStore.createIndex('title', 'title');
        db.createObjectStore('bookmark', { keyPath: 'dataId' });
        db.createObjectStore('lastItem');
      }
    });

    legacyDb.close();

    const upgradedDb = await createBooksDb(name);

    try {
      expect([...upgradedDb.objectStoreNames]).toEqual(expect.arrayContaining(expectedModernStores));
    } finally {
      upgradedDb.close();
    }
  });
});
