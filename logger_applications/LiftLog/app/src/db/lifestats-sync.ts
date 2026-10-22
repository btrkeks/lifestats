import { lifestatsSyncQueueSchema } from '@/db/schema';
import { fromLocalDateJSON, toLocalDateJSON } from '@/models/storage/versions/libs';
import { LocalDate } from '@js-joda/core';
import { and, asc, count, eq, sql } from 'drizzle-orm';
import { ExpoSQLiteDatabase } from 'drizzle-orm/expo-sqlite';

/** A day lifestats may be behind on, and which enqueue of it a push read. */
export type QueuedDay = { date: LocalDate; seq: number };

const queue = lifestatsSyncQueueSchema;

/** Returns whether anything was queued. */
export function enqueueSyncDates(tx: ExpoSQLiteDatabase, dates: LocalDate[]): boolean {
  const rows = [...new Set(dates.map(toLocalDateJSON))].map((date) => ({ date }));
  if (!rows.length) {
    return false;
  }
  tx.insert(queue)
    .values(rows)
    .onConflictDoUpdate({ target: queue.date, set: { seq: sql`${queue.seq} + 1` } })
    .run();
  return true;
}

export function readSyncQueue(db: ExpoSQLiteDatabase): QueuedDay[] {
  return db
    .select()
    .from(queue)
    .orderBy(asc(queue.date))
    .all()
    .map((row) => ({ date: fromLocalDateJSON(row.date), seq: row.seq }));
}

export function countSyncQueue(db: ExpoSQLiteDatabase): number {
  return db.select({ n: count() }).from(queue).get()?.n ?? 0;
}

/** Removes the days a push delivered, unless they were queued again since it read them. */
export function ackSyncDates(tx: ExpoSQLiteDatabase, delivered: QueuedDay[]): void {
  for (const { date, seq } of delivered) {
    tx.delete(queue)
      .where(and(eq(queue.date, toLocalDateJSON(date)), eq(queue.seq, seq)))
      .run();
  }
}
