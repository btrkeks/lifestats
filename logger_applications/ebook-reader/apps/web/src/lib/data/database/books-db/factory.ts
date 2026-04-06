/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */

import type { BooksDb } from './versions/books-db';
import type { IDBPDatabase } from 'idb';
import { openDB } from 'idb';
import upgradeBooksDbFromV2 from './versions/v2/upgrade';

type UpgradeDatabase = Pick<IDBPDatabase<BooksDb>, 'objectStoreNames' | 'createObjectStore'>;

export function createBooksDb(name = 'books') {
  return openDB<BooksDb>(name, 7, {
    async upgrade(oldDb, oldVersion, newVersion, transaction) {
      if (oldVersion === 0) {
        createFreshSchemaStores(oldDb);
      } else if (oldVersion === 2) {
        await upgradeBooksDbFromV2(oldDb, oldVersion, newVersion, transaction);
      }

      ensureStorageSourceStore(oldDb);
      ensureStatisticsStores(oldDb);
      ensureMediaStores(oldDb);
      ensureEbookSyncStores(oldDb);
    }
  });
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
  ensureStorageSourceStore(oldDb);
  ensureStatisticsStores(oldDb);
  ensureMediaStores(oldDb);
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

function ensureEbookSyncStores(oldDb: UpgradeDatabase) {
  if (!oldDb.objectStoreNames.contains('ebookSyncState')) {
    oldDb.createObjectStore('ebookSyncState', {
      keyPath: 'dataId'
    });
  }

  if (!oldDb.objectStoreNames.contains('ebookSyncQueue')) {
    const queueStore = oldDb.createObjectStore('ebookSyncQueue', {
      keyPath: 'eventId'
    });

    queueStore.createIndex('nextRetryAt', 'nextRetryAt');
    queueStore.createIndex('dataId', 'dataId');
  }
}
