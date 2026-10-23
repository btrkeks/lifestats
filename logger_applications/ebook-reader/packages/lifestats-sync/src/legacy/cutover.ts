/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { openDB, type DBSchema, type IDBPDatabase } from 'idb';

import type { BlockedBook, Library, LegacySource, LocalBook } from '../api';
import type { BookIdentity } from '../domain/book-key';
import {
  isDue,
  nextDeliveryState,
  ready,
  type DeliveryContext,
  type DeliveryState,
  type FailedAttempt
} from '../domain/delivery-policy';
import { isServerId, type BookKey, type ServerId } from '../domain/ids';
import { untrackable, type DeliveryProblem } from '../domain/problem';
import type { ServerUrl } from '../domain/server-url';
import { postLegacyEvent, provisionLegacy, putLegacyBook, type LegacyEvent } from '../wire/http';

/**
 * TEMPORARY, for one release. Delivers what the retired v7 patch left behind, then binds each
 * of its books to a content key on the server. Rows go through the legacy events route with
 * their original ids, so the server's primary key decides what it already counted. Nothing
 * is converted into spans, and the v7 rows are never changed: each server that answers gets
 * every row, so a test server configured first cannot consume them. Removing this module and
 * its call in the engine is the cleanup.
 */

interface ReceiptRow {
  /** `event:<event_id>` or `claim:<reader data id>`. */
  readonly id: string;
  /** The server that acknowledged it. */
  readonly sentTo: string | null;
  readonly delivery: DeliveryState;
}

interface ReceiptSchema extends DBSchema {
  receipts: { key: string; value: ReceiptRow };
}

interface Receipt {
  readonly sentTo: ServerId | null;
  readonly delivery: DeliveryState;
}

interface Unit {
  readonly id: string;
  readonly readerDataId: number;
  readonly event: LegacyEvent;
}

export interface CutoverReport {
  /** Rows and claims the current server has not acknowledged. */
  readonly pending: number;
  /** Books whose day records must wait for their claim. */
  readonly held: ReadonlySet<BookKey>;
  readonly blocked: readonly BlockedBook[];
}

export interface CutoverTarget {
  readonly fetch: typeof fetch;
  readonly base: ServerUrl;
  readonly serverId: ServerId;
  readonly context: DeliveryContext;
  readonly random: () => number;
}

/** Deterministic, so a resent tail after a lost reply is a duplicate on the server. */
export function tailEventId(readerDataId: number, from: number, to: number): string {
  return `v7-tail-${readerDataId}-${from}-${to}`;
}

function ackedBy(receipt: Receipt | undefined, serverId: ServerId | null): boolean {
  return serverId !== null && receipt?.sentTo === serverId;
}

export class LegacyCutover {
  private readonly receipts: Promise<IDBPDatabase<ReceiptSchema>>;

  constructor(
    private readonly source: LegacySource,
    private readonly library: Library,
    private readonly identityFor: (book: LocalBook) => Promise<BookIdentity | undefined>,
    dbName: string
  ) {
    const receipts = openDB<ReceiptSchema>(dbName, 1, {
      upgrade(database) {
        database.createObjectStore('receipts', { keyPath: 'id' });
      },
      blocking() {
        void receipts.then((db) => db.close());
      }
    });
    this.receipts = receipts;
  }

  async pending(serverId: ServerId | null): Promise<number> {
    const [units, receipts] = await Promise.all([this.units(), this.read()]);
    return this.count(units, receipts, serverId);
  }

  /** Sends what is due to `serverId`. A transport failure stops the run and is returned. */
  async run(
    target: CutoverTarget,
    onClaimed: (book: BookKey) => Promise<void>
  ): Promise<CutoverReport | FailedAttempt> {
    const { fetch, base, serverId, context } = target;
    const [units, receipts] = await Promise.all([this.units(), this.read()]);
    const open = units.filter((unit) => !ackedBy(receipts.get(unit.id), serverId));
    if (open.some((unit) => isDue(receipts.get(unit.id)?.delivery ?? ready, context))) {
      const provisioned = await provisionLegacy(fetch, base);
      if (provisioned.kind !== 'delivered') return provisioned;
    }

    // A rejected row parks itself and holds its book; the other books keep moving.
    const rejected = new Map<number, DeliveryProblem>();
    for (const unit of open) {
      const receipt = receipts.get(unit.id);
      const delivery = receipt?.delivery ?? ready;
      if (!isDue(delivery, context)) {
        if (delivery.kind === 'parked') rejected.set(unit.readerDataId, delivery.problem);
        continue;
      }
      const outcome = await postLegacyEvent(fetch, base, unit.event);
      if (outcome.kind === 'retryable') return outcome;
      if (outcome.kind === 'delivered') {
        await this.write(receipts, unit.id, { sentTo: serverId, delivery: ready });
        continue;
      }
      await this.write(receipts, unit.id, {
        sentTo: receipt?.sentTo ?? null,
        delivery: nextDeliveryState(delivery, outcome, context, target.random)
      });
      rejected.set(unit.readerDataId, outcome.problem);
    }

    const held = new Set<BookKey>();
    const blocked: BlockedBook[] = [];
    const titles = new Map(units.map((unit) => [unit.readerDataId, unit.event.book_title]));
    for (const readerDataId of new Set(units.map((unit) => unit.readerDataId))) {
      const id = `claim:${readerDataId}`;
      const receipt = receipts.get(id);
      if (ackedBy(receipt, serverId)) continue;
      const local = await this.library.book(readerDataId);
      const rejection = rejected.get(readerDataId);
      if (!local) {
        blocked.push({
          title: titles.get(readerDataId) ?? `ttu book ${readerDataId}`,
          problem: rejection ?? { kind: 'not_in_library', readerDataId }
        });
        continue;
      }
      let identity: BookIdentity | undefined;
      try {
        identity = await this.identityFor(local);
      } catch (error) {
        blocked.push({ title: local.title, problem: untrackable(error) });
        continue;
      }
      if (!identity) {
        blocked.push({ title: local.title, problem: untrackable('It has no countable text.') });
        continue;
      }
      const hold = (problem: DeliveryProblem | null) => {
        held.add(identity.key);
        if (problem) blocked.push({ title: local.title, problem });
      };
      if (rejection) {
        hold(rejection);
        continue;
      }
      const delivery = receipt?.delivery ?? ready;
      if (!isDue(delivery, context)) {
        hold(delivery.kind === 'parked' ? delivery.problem : null);
        continue;
      }
      const outcome = await putLegacyBook(fetch, base, readerDataId, {
        book: identity.key,
        title: local.title,
        length: identity.length
      });
      if (outcome.kind === 'retryable') return outcome;
      if (outcome.kind === 'delivered') {
        await this.write(receipts, id, { sentTo: serverId, delivery: ready });
        await onClaimed(identity.key);
        continue;
      }
      await this.write(receipts, id, {
        sentTo: receipt?.sentTo ?? null,
        delivery: nextDeliveryState(delivery, outcome, context, target.random)
      });
      hold(outcome.problem);
    }
    return { pending: this.count(units, receipts, serverId), held, blocked };
  }

  /** Retry now: parked rows and claims are due again. */
  async rearm(): Promise<void> {
    const db = await this.receipts;
    const tx = db.transaction('receipts', 'readwrite');
    for (const row of await tx.store.getAll()) {
      if (row.delivery.kind !== 'ready') await tx.store.put({ ...row, delivery: ready });
    }
    await tx.done;
  }

  close(): void {
    void this.receipts.then((db) => db.close()).catch(() => undefined);
  }

  private count(
    units: readonly Unit[],
    receipts: ReadonlyMap<string, Receipt>,
    serverId: ServerId | null
  ): number {
    const rows = units.filter((unit) => !ackedBy(receipts.get(unit.id), serverId)).length;
    const claims = [...new Set(units.map((unit) => unit.readerDataId))].filter(
      (readerDataId) => !ackedBy(receipts.get(`claim:${readerDataId}`), serverId)
    ).length;
    return rows + claims;
  }

  private async units(): Promise<Unit[]> {
    const [queued, tails] = await Promise.all([this.source.queued(), this.source.tails()]);
    const titles = new Map(queued.map((row) => [row.readerDataId, row.title]));
    const units: Unit[] = [...queued]
      .sort((a, b) => a.createdAt - b.createdAt)
      .map((row) => ({
        id: `event:${row.eventId}`,
        readerDataId: row.readerDataId,
        event: {
          event_id: row.eventId,
          date: row.day,
          metric: 'characters_read',
          delta: row.delta,
          book_id: row.readerDataId,
          book_title: row.title,
          confirmed_position_after: row.positionAfter,
          book_target_characters: row.targetCharacters || Math.max(row.positionAfter, 1)
        }
      }));
    for (const tail of tails) {
      if (tail.confirmedPosition <= tail.flushedPosition) continue;
      const book = await this.source.book(tail.readerDataId);
      const eventId = tailEventId(tail.readerDataId, tail.flushedPosition, tail.confirmedPosition);
      units.push({
        id: `event:${eventId}`,
        readerDataId: tail.readerDataId,
        event: {
          event_id: eventId,
          date: this.source.dayOf(tail.updatedAt),
          metric: 'characters_read',
          delta: tail.confirmedPosition - tail.flushedPosition,
          book_id: tail.readerDataId,
          book_title:
            book?.title ?? titles.get(tail.readerDataId) ?? `ttu book ${tail.readerDataId}`,
          confirmed_position_after: tail.confirmedPosition,
          book_target_characters: book?.characters || Math.max(tail.confirmedPosition, 1)
        }
      });
    }
    return units;
  }

  private async read(): Promise<Map<string, Receipt>> {
    const rows = await (await this.receipts).getAll('receipts');
    return new Map(
      rows.map((row) => [
        row.id,
        { sentTo: isServerId(row.sentTo) ? row.sentTo : null, delivery: row.delivery }
      ])
    );
  }

  private async write(receipts: Map<string, Receipt>, id: string, receipt: Receipt) {
    await (await this.receipts).put('receipts', { id, ...receipt });
    receipts.set(id, receipt);
  }
}
