/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { openDB, type DBSchema, type IDBPDatabase } from 'idb';

import type { BookIdentity } from '../domain/book-key';
import { absorb, acknowledge, type DayObservation, type DayRecord } from '../domain/day-record';
import { ready, type DeliveryState } from '../domain/delivery-policy';
import {
  isBookKey,
  isProducerId,
  isReadingDay,
  isServerId,
  newProducerId,
  type BookKey,
  type ProducerId,
  type ReadingDay,
  type ServerId
} from '../domain/ids';
import type { DeliveryProblem } from '../domain/problem';
import { parseServerUrl, type ServerUrl } from '../domain/server-url';
import { SpanSet } from '../domain/span-set';

export const SYNC_DB_NAME = 'ttu-lifestats';

interface ProducerRow {
  readonly producerId: string;
}
interface ConfigRow {
  readonly serverUrl: string | null;
  readonly revision: number;
  readonly serverId: string | null;
}
interface DeliveredRow {
  readonly at: number;
}
interface DayRow {
  readonly book: string;
  readonly day: string;
  readonly title: string;
  readonly length: number;
  readonly spans: [number, number][];
  readonly lastPosition: number;
  readonly observedAt: number;
  readonly revision: number;
  readonly delivered: { readonly revision: number; readonly serverId: string } | null;
  readonly delivery: DeliveryState;
}
interface IdentityRow {
  readonly localId: number;
  readonly modified: number;
  readonly key: string;
  readonly length: number;
}
interface ClaimRow {
  readonly readerDataId: number;
  readonly book: string | null;
  readonly ackedBy: string | null;
  readonly missing: boolean;
  readonly problem: DeliveryProblem | null;
}

interface SyncSchema extends DBSchema {
  meta: { key: 'producer' | 'config' | 'delivered'; value: ProducerRow | ConfigRow | DeliveredRow };
  days: { key: [string, string]; value: DayRow };
  identities: { key: number; value: IdentityRow };
  claims: { key: number; value: ClaimRow };
}

export interface SyncConfig {
  readonly serverUrl: ServerUrl | null;
  readonly revision: number;
  readonly serverId: ServerId | null;
}

/** A v7 book that must be bound to its content key on the server before its new reports. */
export interface LegacyClaim {
  readonly readerDataId: number;
  readonly book: BookKey | null;
  readonly ackedBy: ServerId | null;
  readonly missing: boolean;
  readonly problem: DeliveryProblem | null;
}

function toRow(record: DayRecord): DayRow {
  return { ...record, spans: record.spans.toPairs() };
}

function fromRow(row: DayRow): DayRecord | undefined {
  const spans = SpanSet.fromPairs(row.spans);
  if (!spans || !isBookKey(row.book) || !isReadingDay(row.day)) return undefined;
  const delivered =
    row.delivered && isServerId(row.delivered.serverId)
      ? { revision: row.delivered.revision, serverId: row.delivered.serverId }
      : null;
  return { ...row, book: row.book, day: row.day, spans, delivered };
}

function fromClaimRow(row: ClaimRow): LegacyClaim {
  return {
    ...row,
    book: isBookKey(row.book) ? row.book : null,
    ackedBy: isServerId(row.ackedBy) ? row.ackedBy : null
  };
}

const defaultConfig: ConfigRow = { serverUrl: null, revision: 0, serverId: null };

function parseConfig(row: ProducerRow | ConfigRow | DeliveredRow | undefined): SyncConfig {
  const config = row && 'revision' in row ? row : defaultConfig;
  const url = config.serverUrl === null ? null : parseServerUrl(config.serverUrl);
  return {
    serverUrl: url?.ok ? url.url : null,
    revision: config.revision,
    serverId: isServerId(config.serverId) ? config.serverId : null
  };
}

export type SyncStore = Awaited<ReturnType<typeof openSyncStore>>;

/**
 * Lifestats state in its own database, so ttu's `books` schema stays upstream's. Same
 * origin as the books, so it lives exactly as long as they do.
 */
export async function openSyncStore(name = SYNC_DB_NAME) {
  const db: IDBPDatabase<SyncSchema> = await openDB<SyncSchema>(name, 1, {
    upgrade(database) {
      database.createObjectStore('meta');
      database.createObjectStore('days', { keyPath: ['book', 'day'] });
      database.createObjectStore('identities', { keyPath: 'localId' });
      database.createObjectStore('claims', { keyPath: 'readerDataId' });
    },
    blocking() {
      db.close();
    }
  });
  const channel = typeof BroadcastChannel === 'function' ? new BroadcastChannel(name) : null;
  const listeners = new Set<() => void>();
  const changed = () => {
    channel?.postMessage('changed');
    listeners.forEach((listener) => listener());
  };
  if (channel) channel.onmessage = () => listeners.forEach((listener) => listener());

  async function updateDay(
    book: BookKey,
    day: ReadingDay,
    update: (record: DayRecord) => DayRecord
  ): Promise<void> {
    const tx = db.transaction('days', 'readwrite');
    const row = await tx.store.get([book, day]);
    const record = row && fromRow(row);
    if (record) await tx.store.put(toRow(update(record)));
    await tx.done;
    changed();
  }

  return {
    async producer(): Promise<ProducerId> {
      const tx = db.transaction('meta', 'readwrite');
      const row = await tx.store.get('producer');
      const existing = row && 'producerId' in row ? row.producerId : undefined;
      if (isProducerId(existing)) {
        await tx.done;
        return existing;
      }
      const producerId = newProducerId();
      await tx.store.put({ producerId }, 'producer');
      await tx.done;
      return producerId;
    },

    async config(): Promise<SyncConfig> {
      return parseConfig(await db.get('meta', 'config'));
    },

    async setServerUrl(serverUrl: ServerUrl | null): Promise<SyncConfig> {
      const tx = db.transaction('meta', 'readwrite');
      const current = parseConfig(await tx.store.get('config'));
      const next: ConfigRow = { ...current, serverUrl, revision: current.revision + 1 };
      await tx.store.put(next, 'config');
      await tx.done;
      changed();
      return parseConfig(next);
    },

    async setServerId(serverId: ServerId): Promise<void> {
      const tx = db.transaction('meta', 'readwrite');
      const current = parseConfig(await tx.store.get('config'));
      await tx.store.put({ ...current, serverId }, 'config');
      await tx.done;
      changed();
    },

    async lastDeliveredAt(): Promise<number | null> {
      const row = await db.get('meta', 'delivered');
      return row && 'at' in row ? row.at : null;
    },

    /** One transaction: read each record, union the observation in, write only what changed. */
    async commit(observations: readonly DayObservation[]): Promise<void> {
      const tx = db.transaction('days', 'readwrite');
      for (const observation of observations) {
        const row = await tx.store.get([observation.book, observation.day]);
        const next = absorb(row && fromRow(row), observation);
        if (next) await tx.store.put(toRow(next));
      }
      await tx.done;
      changed();
    },

    async records(): Promise<DayRecord[]> {
      const rows = await db.getAll('days');
      return rows.map(fromRow).filter((record): record is DayRecord => record !== undefined);
    },

    async acknowledge(record: DayRecord, serverId: ServerId, at: number): Promise<void> {
      await updateDay(record.book, record.day, (current) =>
        acknowledge(current, record.revision, serverId)
      );
      await db.put('meta', { at }, 'delivered');
    },

    /** Ignored when reading arrived during the attempt: the new revision deserves its own try. */
    async fail(record: DayRecord, delivery: DeliveryState): Promise<void> {
      await updateDay(record.book, record.day, (current) =>
        current.revision === record.revision ? { ...current, delivery } : current
      );
    },

    /** Makes parked and retrying records due now. */
    async rearm(matches: (record: DayRecord) => boolean = () => true): Promise<void> {
      const tx = db.transaction('days', 'readwrite');
      for (const row of await tx.store.getAll()) {
        const record = fromRow(row);
        if (record && record.delivery.kind !== 'ready' && matches(record)) {
          await tx.store.put(toRow({ ...record, delivery: ready }));
        }
      }
      await tx.done;
      changed();
    },

    async identity(localId: number, modified: number): Promise<BookIdentity | undefined> {
      const row = await db.get('identities', localId);
      return row && row.modified === modified && isBookKey(row.key)
        ? { key: row.key, length: row.length }
        : undefined;
    },

    async putIdentity(localId: number, modified: number, identity: BookIdentity): Promise<void> {
      await db.put('identities', { localId, modified, ...identity });
    },

    async addClaims(readerDataIds: readonly number[]): Promise<void> {
      const tx = db.transaction('claims', 'readwrite');
      for (const readerDataId of readerDataIds) {
        if (!(await tx.store.get(readerDataId))) {
          await tx.store.put({
            readerDataId,
            book: null,
            ackedBy: null,
            missing: false,
            problem: null
          });
        }
      }
      await tx.done;
    },

    async claims(): Promise<LegacyClaim[]> {
      return (await db.getAll('claims')).map(fromClaimRow);
    },

    async putClaim(claim: LegacyClaim): Promise<void> {
      await db.put('claims', claim);
      changed();
    },

    onChange(listener: () => void): () => void {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },

    close(): void {
      channel?.close();
      db.close();
    }
  };
}
