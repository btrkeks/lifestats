/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */

import type BooksDb from './versions/books-db';
import type { IDBPDatabase } from 'idb';
import { BehaviorSubject } from 'rxjs';
import { openDB } from 'idb';
import upgradeBooksDbFromV2 from './versions/v2/upgrade';

type UpgradeDatabase = Pick<IDBPDatabase<BooksDb>, 'objectStoreNames' | 'createObjectStore'>;

// Fork: two above upstream's schema version. Laptop databases already reached 7 through
// the retired Lifestats patch, whose ebookSyncState/ebookSyncQueue stores survive here
// untouched until the Lifestats drain empties them. Opening at 8 also fences tabs still
// running the old code: their v7 connection blocks this upgrade until they close.
export const BOOKS_DB_VERSION = 8;

export const booksDbUpgradeBlocked$ = new BehaviorSubject(false);

export function createBooksDb(name = 'books') {
  return openDB<BooksDb>(name, BOOKS_DB_VERSION, {
    async upgrade(oldDb, oldVersion, newVersion, transaction) {
      if (oldVersion === 0) {
        createFreshSchemaStores(oldDb);
      } else if (oldVersion === 2) {
        await upgradeBooksDbFromV2(oldDb, oldVersion, newVersion, transaction);
      }

      // Upstream's switch has no fall-through, so a v4 database would skip the v6 stores.
      ensureStorageSourceStore(oldDb);
      ensureStatisticsStores(oldDb);
      ensureMediaStores(oldDb);
    },
    blocked() {
      booksDbUpgradeBlocked$.next(true);
    },
    blocking(_currentVersion, _blockedVersion, event) {
      (event.target as IDBDatabase).close();
      globalThis.location?.reload();
    }
  }).finally(() => booksDbUpgradeBlocked$.next(false));
}

function createFreshSchemaStores(oldDb: UpgradeDatabase) {
  const dataStore = oldDb.createObjectStore('data', {
    keyPath: 'id',
    autoIncrement: true
  });
  dataStore.createIndex('title', 'title');

  oldDb.createObjectStore('bookmark', {
    keyPath: 'dataId'
  });

  oldDb.createObjectStore('lastItem');
}

function ensureStorageSourceStore(oldDb: UpgradeDatabase) {
  if (!oldDb.objectStoreNames.contains('storageSource')) {
    oldDb.createObjectStore('storageSource', {
      keyPath: 'name'
    });
  }
}

function ensureStatisticsStores(oldDb: UpgradeDatabase) {
  if (!oldDb.objectStoreNames.contains('statistic')) {
    const statisticsStore = oldDb.createObjectStore('statistic', {
      keyPath: ['title', 'dateKey']
    });

    statisticsStore.createIndex('dateKey', 'dateKey');
    statisticsStore.createIndex('completedBook', ['completedBook', 'title']);
  }

  if (!oldDb.objectStoreNames.contains('readingGoal')) {
    const readingGoalsStore = oldDb.createObjectStore('readingGoal', {
      keyPath: 'goalStartDate'
    });

    readingGoalsStore.createIndex('goalEndDate', 'goalEndDate');
  }

  if (!oldDb.objectStoreNames.contains('lastModified')) {
    oldDb.createObjectStore('lastModified', {
      keyPath: ['title', 'dataType']
    });
  }
}

function ensureMediaStores(oldDb: UpgradeDatabase) {
  if (!oldDb.objectStoreNames.contains('audioBook')) {
    oldDb.createObjectStore('audioBook', { keyPath: 'title' });
  }

  if (!oldDb.objectStoreNames.contains('subtitle')) {
    oldDb.createObjectStore('subtitle', { keyPath: 'title' });
  }

  if (!oldDb.objectStoreNames.contains('handle')) {
    oldDb.createObjectStore('handle', { keyPath: ['title', 'dataType'] });
  }
}
