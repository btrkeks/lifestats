/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { wrap, type DBSchema, type IDBPDatabase } from 'idb';

import type { AttemptOutcome } from '../domain/delivery-policy';
import type { LegacyEvent } from '../wire/http';

/**
 * TEMPORARY, until the laptop shows an empty drain. Rows the retired v7 patch left in the
 * `books` database are sent through the legacy events route with their original ids, so
 * the server's primary key decides what it already counted. Nothing is converted into
 * spans. A row is deleted only after the server accepted it, so every run resumes where
 * the last one stopped.
 */

interface V7State {
  readonly dataId: number;
  readonly confirmedPosition: number;
  readonly lastFlushedPosition: number;
  readonly updatedAt: number;
}

interface V7QueueRow {
  readonly eventId: string;
  readonly dataId: number;
  readonly dateKey: string;
  readonly delta: number;
  readonly confirmedPositionAfter: number;
  readonly bookTargetCharacters?: number;
  readonly bookTitle: string;
  readonly createdAt: number;
}

interface V7Schema extends DBSchema {
  data: {
    key: number;
    value: { readonly id: number; readonly title: string; readonly characters: number };
  };
  ebookSyncState: { key: number; value: V7State };
  ebookSyncQueue: { key: string; value: V7QueueRow };
}

export interface LegacyTransport {
  provision(): Promise<AttemptOutcome>;
  send(event: LegacyEvent): Promise<AttemptOutcome>;
}

function hasV7Stores(db: IDBPDatabase<V7Schema>): boolean {
  return (
    db.objectStoreNames.contains('ebookSyncQueue') && db.objectStoreNames.contains('ebookSyncState')
  );
}

/** Reader ids still holding v7 rows, and how many rows remain. */
export async function legacyBacklog(raw: IDBDatabase) {
  const db = wrap(raw) as IDBPDatabase<V7Schema>;
  if (!hasV7Stores(db)) return { readerDataIds: [], rows: 0 };
  const [queue, states] = await Promise.all([
    db.getAll('ebookSyncQueue'),
    db.getAll('ebookSyncState')
  ]);
  const readerDataIds = [...new Set([...queue, ...states].map((row) => row.dataId))];
  return { readerDataIds, rows: queue.length + states.length };
}

/** Deterministic, so a resent tail after a lost reply is a duplicate on the server. */
export function tailEventId(state: V7State): string {
  return `v7-tail-${state.dataId}-${state.lastFlushedPosition}-${state.confirmedPosition}`;
}

export async function drainLegacy(
  raw: IDBDatabase,
  transport: LegacyTransport,
  legacyDayOf: (atMs: number) => string
): Promise<AttemptOutcome> {
  const db = wrap(raw) as IDBPDatabase<V7Schema>;
  if (!hasV7Stores(db)) return { kind: 'delivered' };
  const queue = (await db.getAll('ebookSyncQueue')).sort((a, b) => a.createdAt - b.createdAt);
  const states = await db.getAll('ebookSyncState');
  if (!queue.length && !states.length) return { kind: 'delivered' };

  const provisioned = await transport.provision();
  if (provisioned.kind !== 'delivered') return provisioned;

  const books = new Map((await db.getAll('data')).map((book) => [book.id, book]));
  const titles = new Map(queue.map((row) => [row.dataId, row.bookTitle]));

  for (const row of queue) {
    const outcome = await transport.send({
      event_id: row.eventId,
      date: row.dateKey,
      metric: 'characters_read',
      delta: row.delta,
      book_id: row.dataId,
      book_title: row.bookTitle,
      confirmed_position_after: row.confirmedPositionAfter,
      book_target_characters: row.bookTargetCharacters ?? Math.max(row.confirmedPositionAfter, 1)
    });
    if (outcome.kind !== 'delivered') return outcome;
    await db.delete('ebookSyncQueue', row.eventId);
  }

  for (const state of states) {
    if (state.confirmedPosition > state.lastFlushedPosition) {
      const book = books.get(state.dataId);
      const outcome = await transport.send({
        event_id: tailEventId(state),
        date: legacyDayOf(state.updatedAt),
        metric: 'characters_read',
        delta: state.confirmedPosition - state.lastFlushedPosition,
        book_id: state.dataId,
        book_title: book?.title ?? titles.get(state.dataId) ?? `ttu book ${state.dataId}`,
        confirmed_position_after: state.confirmedPosition,
        book_target_characters: book?.characters || Math.max(state.confirmedPosition, 1)
      });
      if (outcome.kind !== 'delivered') return outcome;
    }
    await db.delete('ebookSyncState', state.dataId);
  }
  return { kind: 'delivered' };
}
