import { describe, expect, it } from 'vitest';
import { LocalDate } from '@js-joda/core';
import { ackSyncDates, enqueueSyncDates, readSyncQueue } from '@/db/lifestats-sync';
import { createTestDb } from '@/utils/__test__/test-db';

const day = (date: string) => LocalDate.parse(date);
const queued = (db: Awaited<ReturnType<typeof createTestDb>>) =>
  readSyncQueue(db).map(({ date, seq }) => [date.toString(), seq]);

describe('lifestats sync queue', () => {
  it('bumps the seq of a day queued again', async () => {
    const db = await createTestDb();

    db.transaction((tx) => enqueueSyncDates(tx, [day('2026-10-09'), day('2026-10-11'), day('2026-10-09')]));
    db.transaction((tx) => enqueueSyncDates(tx, [day('2026-10-09')]));

    expect(queued(db)).toEqual([
      ['2026-10-09', 2],
      ['2026-10-11', 1],
    ]);
  });

  it('keeps a day queued again after the push read it', async () => {
    const db = await createTestDb();
    db.transaction((tx) => enqueueSyncDates(tx, [day('2026-10-09'), day('2026-10-10')]));
    const read = readSyncQueue(db);

    db.transaction((tx) => enqueueSyncDates(tx, [day('2026-10-10')]));
    db.transaction((tx) => ackSyncDates(tx, read));

    expect(queued(db)).toEqual([['2026-10-10', 2]]);
  });
});
