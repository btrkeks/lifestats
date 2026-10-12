import { describe, expect, it } from 'vitest';
import { LocalDate } from '@js-joda/core';
import { ackSyncDates, enqueueSyncDates, queueDaysChangedBy, readSyncQueue } from '@/db/lifestats-sync';
import { writeSession } from '@/db/sessions';
import { makeWorkoutOn } from '@/models/session-models/__test__/helpers';
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

  it('queues the old and new day of a moved workout', async () => {
    const db = await createTestDb([makeWorkoutOn('2026-10-09')]);
    const moved = makeWorkoutOn('2026-10-09').withUpdatedDate(day('2026-10-10'));

    db.transaction((tx) => queueDaysChangedBy(tx, [moved.id], () => writeSession(tx, moved)));

    expect(queued(db)).toEqual([
      ['2026-10-09', 1],
      ['2026-10-10', 1],
    ]);
  });

  it('rolls the queue back with a write that fails', async () => {
    const db = await createTestDb([makeWorkoutOn('2026-10-09')]);
    const moved = makeWorkoutOn('2026-10-09').withUpdatedDate(day('2026-10-10'));

    expect(() =>
      db.transaction((tx) =>
        queueDaysChangedBy(tx, [moved.id], () => {
          writeSession(tx, moved);
          throw new Error('disk full');
        }),
      ),
    ).toThrow('disk full');

    expect(queued(db)).toEqual([]);
    db.transaction((tx) => queueDaysChangedBy(tx, [moved.id], () => writeSession(tx, moved)));
    expect(queued(db)).toEqual([
      ['2026-10-09', 1],
      ['2026-10-10', 1],
    ]);
  });
});
