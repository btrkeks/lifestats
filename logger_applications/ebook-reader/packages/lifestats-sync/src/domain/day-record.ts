/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { ready, type DeliveryState } from './delivery-policy';
import type { BookKey, ReadingDay, ServerId } from './ids';
import type { SpanSet } from './span-set';

/**
 * One (book, day) on this producer: what was read and whether the server has it, in one
 * row, so no separate outbox can drift from the truth.
 */
export interface DayRecord {
  readonly book: BookKey;
  readonly day: ReadingDay;
  readonly title: string;
  readonly length: number;
  readonly spans: SpanSet;
  readonly lastPosition: number;
  readonly observedAt: number;
  /** Increments on every change that the server should see. */
  readonly revision: number;
  /** The highest revision a server acknowledged, and which server. */
  readonly delivered: { readonly revision: number; readonly serverId: ServerId } | null;
  readonly delivery: DeliveryState;
}

/** What one accepted observation adds to its (book, day). */
export interface DayObservation {
  readonly book: BookKey;
  readonly day: ReadingDay;
  readonly title: string;
  readonly length: number;
  readonly spans: SpanSet;
  readonly position: number;
  readonly atMs: number;
}

/**
 * Merges an observation into the stored record. Returns undefined when nothing changed,
 * so applying the same observation twice writes once.
 */
export function absorb(
  existing: DayRecord | undefined,
  observation: DayObservation
): DayRecord | undefined {
  if (!existing) {
    return {
      book: observation.book,
      day: observation.day,
      title: observation.title,
      length: observation.length,
      spans: observation.spans,
      lastPosition: observation.position,
      observedAt: observation.atMs,
      revision: 1,
      delivered: null,
      delivery: ready
    };
  }
  const spans = existing.spans.union(observation.spans);
  const newer = observation.atMs > existing.observedAt;
  const moved = newer && observation.position !== existing.lastPosition;
  if (spans.equals(existing.spans) && !moved) return undefined;
  return {
    ...existing,
    title: newer ? observation.title : existing.title,
    spans,
    lastPosition: moved ? observation.position : existing.lastPosition,
    observedAt: moved ? observation.atMs : existing.observedAt,
    revision: existing.revision + 1,
    delivery: existing.delivery.kind === 'parked' ? ready : existing.delivery
  };
}

/** Never lowers the acknowledged revision, so a late reply cannot un-deliver newer reading. */
export function acknowledge(record: DayRecord, revision: number, serverId: ServerId): DayRecord {
  const previous = record.delivered?.serverId === serverId ? record.delivered.revision : 0;
  return {
    ...record,
    delivered: { revision: Math.max(previous, revision), serverId },
    delivery: ready
  };
}

/** A record is pending until the current server acknowledged its current revision. */
export function isPending(record: DayRecord, serverId: ServerId | null): boolean {
  return (
    record.delivered === null ||
    record.delivered.serverId !== serverId ||
    record.delivered.revision < record.revision
  );
}
