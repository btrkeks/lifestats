import { describe, it, expect, vi } from 'vitest';
import { drizzle } from 'drizzle-orm/expo-sqlite';
import { deserializeDatabaseAsync, openDatabaseAsync, type SQLiteDatabase } from 'expo-sqlite';
import { gunzipSync } from 'zlib';
import { DatabaseMigrationService } from '@/services/database-migration-service';
import {
  backendAssignmentsSchema,
  backendHeadersSchema,
  backendsSchema,
  feedIdentitySchema,
  calendarClaimsSchema,
  calendarSyncSessionsSchema,
  lifestatsSyncQueueSchema,
} from '@/db/schema';
import { getBackupBytes } from '@/store/settings/util';
import { toLocalDateJSON } from '@/models/storage/versions/libs';
import { LocalDate } from '@js-joda/core';
import 'compression-streams-polyfill';

async function createSeededDb(): Promise<SQLiteDatabase> {
  const expoDb = await openDatabaseAsync(':memory:');
  const db = drizzle(expoDb);
  await new DatabaseMigrationService(db, { info: vi.fn(), error: vi.fn(), warn: vi.fn(), debug: vi.fn() } as never, {
    importOldData: async () => {},
  }).migrate();

  await db
    .insert(backendsSchema)
    .values({ id: 'self', name: 'Home server', url: 'https://liftlog.example.com', kind: 'liftlog' });
  await db.insert(backendHeadersSchema).values({ backendId: 'self', name: 'X-Api-Key', value: 'super-secret' });
  await db.insert(backendAssignmentsSchema).values({ feature: 'backup', backendId: 'self' });
  await db.insert(feedIdentitySchema).values({ id: 0, payload: {} as never });
  await db.insert(lifestatsSyncQueueSchema).values({ date: toLocalDateJSON(LocalDate.of(2026, 10, 11)) });
  await db
    .insert(calendarSyncSessionsSchema)
    .values({ sessionId: 'workout', calendarId: 7, accountName: 'me@example.com', title: 'Gym' });
  await db.insert(calendarClaimsSchema).values({
    sessionId: 'workout',
    calendarId: 7,
    accountName: 'me@example.com',
    eventId: 42,
    instanceTime: 1_791_820_800_000,
    date: toLocalDateJSON(LocalDate.of(2026, 10, 11)),
    originalStartMs: 1_791_820_800_000,
    originalEndMs: 1_791_824_400_000,
    observedStartMs: 1_791_820_800_000,
    observedEndMs: 1_791_824_400_000,
  });

  return expoDb;
}

describe('getBackupBytes', () => {
  // The backup blob is plaintext gzip that lands on the very server whose credentials it would
  // otherwise carry.
  it('strips backend configuration and its secrets', async () => {
    const expoDb = await createSeededDb();

    const gzipped = await getBackupBytes(expoDb);
    const restored = drizzle(await deserializeDatabaseAsync(gunzipSync(gzipped)));

    expect(await restored.select().from(backendsSchema)).toEqual([]);
    expect(await restored.select().from(backendHeadersSchema)).toEqual([]);
    expect(await restored.select().from(backendAssignmentsSchema)).toEqual([]);
  });

  it('strips leftover feed data', async () => {
    const expoDb = await createSeededDb();

    const gzipped = await getBackupBytes(expoDb);
    const restored = drizzle(await deserializeDatabaseAsync(gunzipSync(gzipped)));

    expect(await restored.select().from(feedIdentitySchema)).toEqual([]);
  });

  it('strips the days queued for lifestats', async () => {
    const expoDb = await createSeededDb();

    const gzipped = await getBackupBytes(expoDb);
    const restored = drizzle(await deserializeDatabaseAsync(gunzipSync(gzipped)));

    expect(await restored.select().from(lifestatsSyncQueueSchema)).toEqual([]);
    expect(await drizzle(expoDb).select().from(lifestatsSyncQueueSchema)).toEqual([{ date: '2026-10-11', seq: 1 }]);
  });

  // Calendar and event row ids belong to this phone's calendar provider; on another phone they could name
  // someone else's events.
  it('strips calendar sync workouts and the events they own', async () => {
    const expoDb = await createSeededDb();

    const gzipped = await getBackupBytes(expoDb);
    const restored = drizzle(await deserializeDatabaseAsync(gunzipSync(gzipped)));

    expect(await restored.select().from(calendarClaimsSchema)).toEqual([]);
    expect(await restored.select().from(calendarSyncSessionsSchema)).toEqual([]);
    expect(await drizzle(expoDb).select().from(calendarClaimsSchema)).toHaveLength(1);
  });

  it('leaves the source database untouched', async () => {
    const expoDb = await createSeededDb();

    await getBackupBytes(expoDb);

    expect(await drizzle(expoDb).select().from(backendsSchema)).toHaveLength(1);
  });
});
