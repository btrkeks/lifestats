/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import 'fake-indexeddb/auto';

import { createBooksDb } from '$lib/data/database/books-db/factory';
import { deleteDB, openDB } from 'idb';
import { afterEach, describe, expect, it } from 'vitest';
import { legacySource } from './legacy-source';

const names: string[] = [];

function dbName() {
  const name = `books-${crypto.randomUUID()}`;
  names.push(name);
  return name;
}

afterEach(async () => {
  await Promise.all(names.splice(0).map((name) => deleteDB(name)));
});

const queued = {
  eventId: '0f8fad5b-d9cb-469f-a165-70867728950e',
  dataId: 1,
  dateKey: '2023-11-14',
  metric: 'characters_read',
  delta: 3,
  confirmedPositionAfter: 3,
  bookTitle: '吾輩は猫である',
  createdAt: 1_700_000_250_000,
  attemptCount: 2,
  nextRetryAt: 1_700_000_260_000
};
const state = {
  dataId: 1,
  currentPosition: 5,
  confirmedPosition: 5,
  lastFlushedPosition: 3,
  bundleVersion: 1,
  updatedAt: 1_700_000_300_000
};

async function laptopProfile(name: string) {
  const v7 = await openDB(name, 7, {
    upgrade(db) {
      db.createObjectStore('data', { keyPath: 'id', autoIncrement: true }).createIndex(
        'title',
        'title'
      );
      db.createObjectStore('ebookSyncState', { keyPath: 'dataId' });
      db.createObjectStore('ebookSyncQueue', { keyPath: 'eventId' });
    }
  });
  await v7.put('data', { id: 1, title: '吾輩は猫である', characters: 8 });
  await v7.put('ebookSyncQueue', queued);
  await v7.put('ebookSyncState', state);
  v7.close();
}

async function v7Rows(name: string) {
  const db = await openDB(name);
  try {
    return [await db.getAll('ebookSyncQueue'), await db.getAll('ebookSyncState')];
  } finally {
    db.close();
  }
}

describe('legacySource', () => {
  it('reads the v7 rows of a laptop profile opened at v8 and changes nothing', async () => {
    const name = dbName();
    await laptopProfile(name);
    const books = createBooksDb(name);
    const source = legacySource(books, (atMs) => `day-${atMs}`);

    expect(await source.queued()).toEqual([
      {
        eventId: queued.eventId,
        readerDataId: 1,
        day: '2023-11-14',
        delta: 3,
        positionAfter: 3,
        targetCharacters: null,
        title: '吾輩は猫である',
        createdAt: 1_700_000_250_000
      }
    ]);
    expect(await source.tails()).toEqual([
      { readerDataId: 1, flushedPosition: 3, confirmedPosition: 5, updatedAt: 1_700_000_300_000 }
    ]);
    expect(await source.book(1)).toEqual({ title: '吾輩は猫である', characters: 8 });
    expect(await source.book(2)).toBeUndefined();
    expect(source.dayOf(7)).toBe('day-7');

    (await books).close();
    expect(await v7Rows(name)).toEqual([[queued], [state]]);
  });

  it('finds nothing in a profile that never ran the v7 patch', async () => {
    const books = createBooksDb(dbName());
    const source = legacySource(books, String);

    expect(await source.queued()).toEqual([]);
    expect(await source.tails()).toEqual([]);
    (await books).close();
  });
});
