import { describe, expect, it, vi } from 'vitest';
import { Duration, LocalDate, OffsetDateTime } from '@js-joda/core';
import { SessionBlueprint } from '@/models/blueprint-models';
import { RecordedCardioExercise, Session } from '@/models/session-models';
import {
  makeCardioBlueprint,
  makeRecordedExercise,
  makeWeightedBlueprint,
  makeWorkoutOn,
} from '@/models/session-models/__test__/helpers';
import { enqueueSyncDates, readSyncQueue } from '@/db/lifestats-sync';
import { pushQueuedDays, syncBaseUrl, syncDay } from '@/services/lifestats-sync';
import { createTestDb } from '@/utils/__test__/test-db';

const at = (time: string) => OffsetDateTime.parse(time);

function lifting(id: string, date: string, times: string[]) {
  const blueprint = makeWeightedBlueprint();
  return new Session(
    id,
    new SessionBlueprint('Push', [blueprint], ''),
    [
      makeRecordedExercise(
        blueprint,
        times.map(() => 5),
        undefined,
        (i) => at(times[i]!),
      ),
    ],
    LocalDate.parse(date),
    undefined,
    undefined,
  );
}

function rowing(id: string, date: string, completed: string, minutes: number) {
  const blueprint = makeCardioBlueprint();
  const exercise = RecordedCardioExercise.empty(blueprint).withAllSets((set) =>
    set.with({ completionDateTime: at(completed), duration: Duration.ofMinutes(minutes) }),
  );
  return new Session(
    id,
    new SessionBlueprint('Row', [blueprint], ''),
    [exercise],
    LocalDate.parse(date),
    undefined,
    undefined,
  );
}

describe('syncDay', () => {
  it('sends each finished workout of the day from its first to its last moment, earliest first', () => {
    const sessions = [
      rowing('row', '2026-10-11', '2026-10-11T19:30:00+02:00', 10),
      lifting('lift', '2026-10-11', ['2026-10-11T18:55:40+02:00', '2026-10-11T18:02:11+02:00']),
      lifting('other-day', '2026-10-10', ['2026-10-10T18:00:00+02:00']),
      makeWorkoutOn('2026-10-11').withNothingCompleted(),
    ];

    expect(JSON.stringify(syncDay(LocalDate.parse('2026-10-11'), sessions))).toBe(
      '{"date":"2026-10-11","workouts":[' +
        '{"id":"lift","start":"2026-10-11T18:02:11+02:00","end":"2026-10-11T18:55:40+02:00"},' +
        '{"id":"row","start":"2026-10-11T19:30:00+02:00","end":"2026-10-11T19:40:00+02:00"}]}',
    );
  });

  it('sends a day with no workouts left as empty, so lifestats clears it', () => {
    expect(syncDay(LocalDate.parse('2026-10-09'), [])).toEqual({ date: '2026-10-09', workouts: [] });
  });
});

describe('syncBaseUrl', () => {
  it.each([
    [' https://laptop-1.tail87da75.ts.net/ ', 'https://laptop-1.tail87da75.ts.net'],
    ['https://example.com/lifestats//', 'https://example.com/lifestats'],
    ['http://laptop-1.tail87da75.ts.net', undefined],
    ['https://example.com/?key=1', undefined],
    ['', undefined],
  ])('reads %j as %j', (raw, expected) => {
    expect(syncBaseUrl(raw)).toBe(expected);
  });
});

describe('pushQueuedDays', () => {
  const baseUrl = 'https://lifestats.test';
  const respond = (status: number, body: string) =>
    vi.fn<typeof globalThis.fetch>().mockResolvedValue(new Response(body, { status }));
  const accepted = () => respond(200, '{"statistic_id":129,"days_updated":1,"days_cleared":1,"days_unchanged":0}');

  async function dbWithQueued(dates: string[], sessions: Session[] = []) {
    const db = await createTestDb(sessions);
    db.transaction((tx) =>
      enqueueSyncDates(
        tx,
        dates.map((d) => LocalDate.parse(d)),
      ),
    );
    return db;
  }
  const queued = (db: Awaited<ReturnType<typeof createTestDb>>) => readSyncQueue(db).map((d) => d.date.toString());

  it('replaces the queued days on lifestats and empties the queue', async () => {
    const db = await dbWithQueued(
      ['2026-10-09', '2026-10-11'],
      [lifting('lift', '2026-10-11', ['2026-10-11T18:02:11+02:00', '2026-10-11T18:55:40+02:00'])],
    );
    const fetch = accepted();

    const outcome = await pushQueuedDays({ db, baseUrl, fetch });

    expect(outcome).toEqual({ kind: 'delivered', days: 2 });
    expect(queued(db)).toEqual([]);
    expect(
      fetch.mock.calls.map(([url, init]) => [url, init?.method, JSON.parse(init?.body as string) as unknown]),
    ).toEqual([
      [
        'https://lifestats.test/integrations/liftlog/days',
        'PUT',
        {
          days: [
            { date: '2026-10-09', workouts: [] },
            {
              date: '2026-10-11',
              workouts: [{ id: 'lift', start: '2026-10-11T18:02:11+02:00', end: '2026-10-11T18:55:40+02:00' }],
            },
          ],
        },
      ],
    ]);
  });

  it('sends a hundred days per request', async () => {
    const dates = Array.from({ length: 150 }, (_, i) => LocalDate.of(2026, 1, 1).plusDays(i).toString());
    const db = await dbWithQueued(dates);
    const fetch = accepted();

    await pushQueuedDays({ db, baseUrl, fetch });

    expect(
      fetch.mock.calls.map(([, init]) => (JSON.parse(init?.body as string) as { days: unknown[] }).days.length),
    ).toEqual([100, 50]);
  });

  it.each([
    [
      'a server error',
      respond(503, 'Service Unavailable'),
      { retryable: true, error: 'HTTP 503: Service Unavailable' },
    ],
    [
      'a rejected request',
      respond(400, '{"error":"duplicate date 2026-10-09"}'),
      { retryable: false, error: 'HTTP 400: duplicate date 2026-10-09' },
    ],
    [
      'a 200 from something else',
      respond(200, '<html>Sign in to Wi-Fi</html>'),
      { retryable: false, error: 'Not a lifestats response' },
    ],
    [
      'no network',
      vi.fn<typeof globalThis.fetch>().mockRejectedValue(new TypeError('Network request failed')),
      { retryable: true, error: 'Network request failed' },
    ],
  ])('keeps the days queued after %s', async (_, fetch, failure) => {
    const db = await dbWithQueued(['2026-10-09']);

    const outcome = await pushQueuedDays({ db, baseUrl, fetch });

    expect(outcome).toEqual({ kind: 'failed', days: 0, ...failure });
    expect(queued(db)).toEqual(['2026-10-09']);
  });
});
