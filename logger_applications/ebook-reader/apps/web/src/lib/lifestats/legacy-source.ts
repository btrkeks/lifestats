/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type BooksDb from '$lib/data/database/books-db/versions/books-db';
import type { LegacySource } from '@lifestats/ttu-sync';
import type { DBSchema, IDBPDatabase } from 'idb';

/** The two stores the retired v7 patch added to `books`, as it wrote them. */
interface V7Stores extends DBSchema {
  ebookSyncQueue: {
    key: string;
    value: {
      readonly eventId: string;
      readonly dataId: number;
      readonly dateKey: string;
      readonly delta: number;
      readonly confirmedPositionAfter: number;
      readonly bookTargetCharacters?: number;
      readonly bookTitle: string;
      readonly createdAt: number;
    };
  };
  ebookSyncState: {
    key: number;
    value: {
      readonly dataId: number;
      readonly confirmedPosition: number;
      readonly lastFlushedPosition: number;
      readonly updatedAt: number;
    };
  };
}

/** TEMPORARY, until the v7 stores are dropped. Reads them without changing anything. */
export function legacySource(
  books: Promise<IDBPDatabase<BooksDb>>,
  dayOf: (atMs: number) => string
): LegacySource {
  async function v7(): Promise<IDBPDatabase<V7Stores> | null> {
    // Upstream's schema type does not know the v7 stores, so their presence is checked here.
    const db = (await books) as unknown as IDBPDatabase<V7Stores>;
    return db.objectStoreNames.contains('ebookSyncQueue') &&
      db.objectStoreNames.contains('ebookSyncState')
      ? db
      : null;
  }

  return {
    async queued() {
      const db = await v7();
      return (
        (await db?.getAll('ebookSyncQueue'))?.map((row) => ({
          eventId: row.eventId,
          readerDataId: row.dataId,
          day: row.dateKey,
          delta: row.delta,
          positionAfter: row.confirmedPositionAfter,
          targetCharacters: row.bookTargetCharacters ?? null,
          title: row.bookTitle,
          createdAt: row.createdAt
        })) ?? []
      );
    },
    async tails() {
      const db = await v7();
      return (
        (await db?.getAll('ebookSyncState'))?.map((state) => ({
          readerDataId: state.dataId,
          flushedPosition: state.lastFlushedPosition,
          confirmedPosition: state.confirmedPosition,
          updatedAt: state.updatedAt
        })) ?? []
      );
    },
    async book(readerDataId) {
      const data = await (await books).get('data', readerDataId);
      return data && { title: data.title, characters: data.characters };
    },
    dayOf
  };
}
