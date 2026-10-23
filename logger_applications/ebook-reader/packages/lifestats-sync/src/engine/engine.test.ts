/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { afterEach, describe, expect, it } from 'vitest';

import type { ReaderBinding, SyncStatus } from '../api';
import { bookIdentity } from '../domain/book-key';
import type { BookKey, ReadingDay } from '../domain/ids';
import { SpanSet } from '../domain/span-set';
import { openSyncStore } from '../store/sync-db';
import {
  BASE,
  Clock,
  FakeLegacy,
  FakeServer,
  book,
  legacyRow,
  realTimers,
  rig,
  started,
  type Rig
} from '../test/harness';

const TEXT = '猫'.repeat(10_000);
const rigs: Rig[] = [];
let counter = 0;

function dbName() {
  counter += 1;
  return `ttu-lifestats-test-${counter}-${crypto.randomUUID()}`;
}

async function open(
  server: FakeServer,
  name = dbName(),
  extra: Partial<Parameters<typeof rig>[0]> = {}
) {
  const r = rig({ server, dbName: name, books: [book(1, TEXT)], ...extra });
  rigs.push(r);
  await started(r.engine);
  return { ...r, name };
}

afterEach(() => rigs.splice(0).forEach((r) => r.engine.close()));

async function key(): Promise<BookKey> {
  const identity = await bookIdentity(TEXT);
  if (!identity) throw new Error('no identity');
  return identity.key;
}

/** Turns pages of 300 characters every 10 s, a pace the policy credits. */
function read(r: Rig, binding: ReaderBinding, from: number, to: number) {
  binding.observe(from);
  for (let position = from + 300; position <= to; position += 300) {
    r.clock.advance(10);
    binding.observe(position);
  }
}

/** Configures and waits for the delivery pass that configuring starts. */
async function configure(r: Rig) {
  await r.engine.configure(BASE);
  await r.engine.drain(5_000);
}

function status(r: Rig): SyncStatus {
  return r.engine.status.get();
}

async function storedSpans(name: string): Promise<SpanSet> {
  const store = await openSyncStore(name);
  try {
    return (await store.records()).reduce((all, record) => all.union(record.spans), SpanSet.empty);
  } finally {
    store.close();
  }
}

async function readerFor(r: Rig) {
  const binding = r.engine.reader({ forwardSkipThreshold: () => 2700 });
  binding.setBook(book(1, TEXT));
  await r.engine.flush();
  return binding;
}

describe('delivery', () => {
  it('delivers the union of what was read, and a second sync changes nothing', async () => {
    const server = new FakeServer();
    const r = await open(server);
    await configure(r);
    const binding = await readerFor(r);
    read(r, binding, 0, 3000);

    expect(await r.engine.drain(5_000)).toBe('idle');
    expect(server.credited(await key())).toBe(3000);
    const puts = server.requests.filter((request) => request.startsWith('PUT')).length;

    expect(await r.engine.drain(5_000)).toBe('idle');
    expect(server.requests.filter((request) => request.startsWith('PUT')).length).toBe(puts);
    expect(status(r).kind).toBe('synced');
  });

  it('records locally while no server is configured', async () => {
    const server = new FakeServer();
    const r = await open(server);
    const binding = await readerFor(r);
    read(r, binding, 0, 1500);

    expect(await r.engine.drain(5_000)).toBe('pending');
    expect(status(r)).toMatchObject({
      kind: 'not_configured',
      pending: { records: 1, characters: 1500 }
    });
    expect(server.requests).toEqual([]);

    await configure(r);
    expect(await r.engine.drain(5_000)).toBe('idle');
    expect(server.credited(await key())).toBe(1500);
  });

  it('waits with a reason and pending count while the server is down, then delivers once', async () => {
    const server = new FakeServer();
    const r = await open(server);
    await configure(r);
    const binding = await readerFor(r);
    server.down = true;
    read(r, binding, 0, 900);

    expect(await r.engine.drain(5_000)).toBe('pending');
    expect(status(r)).toMatchObject({
      kind: 'waiting',
      problem: { kind: 'unreachable', serverUrl: BASE },
      pending: { records: 1, characters: 900 }
    });

    server.down = false;
    await r.engine.retryNow();
    expect(status(r).kind).toBe('synced');
    expect(server.credited(await key())).toBe(900);
  });

  it('retries 5xx with backoff and does not resend before the backoff ends', async () => {
    const server = new FakeServer();
    const r = await open(server);
    await configure(r);
    const binding = await readerFor(r);
    server.answer.set('/days/', 503);
    read(r, binding, 0, 600);
    await r.engine.drain(5_000);
    expect(status(r)).toMatchObject({
      kind: 'waiting',
      problem: { kind: 'server_error', httpStatus: 503 }
    });

    const waiting = status(r);
    if (waiting.kind !== 'waiting') throw new Error('expected waiting');
    expect(waiting.nextAttemptAt - r.clock.now()).toBeGreaterThanOrEqual(4_000);

    server.answer.clear();
    r.clock.ms = waiting.nextAttemptAt - 1;
    const before = server.requests.length;
    await r.engine.drain(5_000);
    expect(server.requests.length).toBe(before);

    r.clock.ms = waiting.nextAttemptAt;
    await r.engine.drain(5_000);
    expect(server.credited(await key())).toBe(600);
  });

  it('parks a rejected record until a new build, a config change, Retry now, or 6 hours', async () => {
    const server = new FakeServer();
    const name = dbName();
    const r = await open(server, name);
    await configure(r);
    const binding = await readerFor(r);
    server.answer.set('/days/', 422);
    read(r, binding, 0, 600);
    await r.engine.drain(5_000);
    expect(status(r)).toMatchObject({
      kind: 'blocked',
      problem: { kind: 'rejected', code: 'invalid_field' }
    });

    server.answer.clear();
    await r.engine.drain(5_000);
    expect(server.credited(await key())).toBe(0);

    r.clock.advance(6 * 3600);
    await r.engine.drain(5_000);
    expect(server.credited(await key())).toBe(600);
  });

  it.each([
    [
      'a new build',
      async (r: Rig, name: string, server: FakeServer) => {
        r.engine.close();
        const next = await open(server, name, { buildId: 'build-2', clock: r.clock });
        await next.engine.drain(5_000);
      }
    ],
    [
      'a config change',
      async (r: Rig) => {
        await configure(r);
        await r.engine.drain(5_000);
      }
    ],
    [
      'Retry now',
      async (r: Rig) => {
        await r.engine.retryNow();
      }
    ]
  ])('re-arms a parked record after %s', async (_label, rearm) => {
    const server = new FakeServer();
    const name = dbName();
    const r = await open(server, name);
    await configure(r);
    const binding = await readerFor(r);
    server.answer.set('/days/', 422);
    read(r, binding, 0, 600);
    await r.engine.drain(5_000);
    await r.engine.flush();
    server.answer.clear();

    await rearm(r, name, server);
    expect(server.credited(await key())).toBe(600);
  });

  it('resends a record whose acknowledgement was lost, without counting it twice', async () => {
    const server = new FakeServer();
    const name = dbName();
    const r = await open(server, name);
    await configure(r);
    const binding = await readerFor(r);
    read(r, binding, 0, 1200);
    server.loseReplies = 1;
    server.loseRepliesTo = '/days/';
    await r.engine.drain(5_000);
    expect(status(r).kind).toBe('waiting');
    r.engine.close();

    const reopened = await open(server, name, { clock: r.clock });
    await reopened.engine.retryNow();
    expect(status(reopened).kind).toBe('synced');
    expect(server.credited(await key())).toBe(1200);
  });

  it('sends everything again when the server identity changes', async () => {
    const server = new FakeServer();
    const r = await open(server);
    await configure(r);
    const binding = await readerFor(r);
    read(r, binding, 0, 900);
    await r.engine.drain(5_000);

    server.serverId = '5b0c1f8e-2f4d-4c43-9a43-7c1fd0b5a002';
    server.days.clear();
    r.clock.advance(10);
    read(r, binding, 900, 1500);
    await r.engine.drain(5_000);

    expect(server.credited(await key())).toBe(1500);
  });

  it('replays old days to a new server identity after later deliveries', async () => {
    const server = new FakeServer();
    const clock = new Clock(Date.parse('2026-09-01T10:00:00'));
    const r = await open(server, dbName(), { clock });
    await configure(r);
    const binding = await readerFor(r);
    read(r, binding, 0, 900);
    expect(await r.engine.drain(5_000)).toBe('idle');

    clock.ms = Date.parse('2026-10-22T10:00:00');
    read(r, binding, 900, 1500);
    expect(await r.engine.drain(5_000)).toBe('idle');

    server.serverId = '5b0c1f8e-2f4d-4c43-9a43-7c1fd0b5a002';
    server.days.clear();
    await r.engine.retryNow();

    expect(server.credited(await key())).toBe(1500);
    expect([...server.days.keys()].some((day) => day.endsWith('|2026-09-01'))).toBe(true);
  });
});

describe('recording', () => {
  it('commits exactly the union of 50 overlapping observations', async () => {
    const r = await open(new FakeServer());
    const book = await key();
    let expected = SpanSet.empty;
    for (let i = 0; i < 50; i += 1) {
      const spans = SpanSet.span((i * 37) % 900, ((i * 37) % 900) + 150);
      expected = expected.union(spans);
      r.engine.record({
        book,
        day: '2026-10-22' as ReadingDay,
        title: 'Book 1',
        length: 10_000,
        spans,
        position: 0,
        atMs: i
      });
    }
    await r.engine.flush();
    expect((await storedSpans(r.name)).toPairs()).toEqual(expected.toPairs());
  });

  it('keeps one record with the union when two tabs read the same pages', async () => {
    const server = new FakeServer();
    const name = dbName();
    const first = await open(server, name);
    const second = await open(server, name, { clock: first.clock });
    const a = await readerFor(first);
    const b = await readerFor(second);
    a.observe(0);
    b.observe(0);
    for (let position = 300; position <= 3000; position += 300) {
      first.clock.advance(10);
      a.observe(position);
      b.observe(position);
    }
    await Promise.all([first.engine.flush(), second.engine.flush()]);

    expect((await storedSpans(name)).toPairs()).toEqual([[0, 3000]]);
    await configure(first);
    await first.engine.drain(5_000);
    await second.engine.drain(5_000);
    expect(server.credited(await key())).toBe(3000);
  });

  it('keeps everything observed before the last completed commit after a hard kill', async () => {
    const name = dbName();
    const r = await open(new FakeServer(), name);
    const binding = await readerFor(r);
    read(r, binding, 0, 2400);
    await r.engine.flush();
    r.engine.close();

    expect((await storedSpans(name)).toPairs()).toEqual([[0, 2400]]);
  });

  it('credits nothing for a restore, a resize, or a skip', async () => {
    const r = await open(new FakeServer());
    const binding = await readerFor(r);
    r.engine.relocate();
    binding.observe(0);
    binding.observe(5000);
    r.clock.advance(60);
    binding.observe(5300);
    r.clock.advance(10);
    r.engine.relocate();
    binding.observe(5450);
    r.clock.advance(60);
    binding.observe(9999);
    await r.engine.flush();

    expect((await storedSpans(r.name)).toPairs()).toEqual([[5000, 5300]]);
  });

  it('credits the first page turned after opening a book', async () => {
    const r = await open(new FakeServer());
    const binding = r.engine.reader({ forwardSkipThreshold: () => 2700 });
    binding.observe(0);
    binding.setBook(book(1, TEXT));
    await r.engine.flush();
    r.clock.advance(30);
    binding.observe(700);
    await r.engine.flush();

    expect((await storedSpans(r.name)).toPairs()).toEqual([[0, 700]]);
  });

  it('credits a page turn into the next section despite the layout reset to 0', async () => {
    const r = await open(new FakeServer());
    const binding = await readerFor(r);
    read(r, binding, 2000, 2600);
    r.clock.advance(10);
    binding.observe(0);
    binding.observe(2900);
    r.clock.advance(10);
    binding.observe(3200);
    await r.engine.flush();

    expect((await storedSpans(r.name)).toPairs()).toEqual([[2000, 3200]]);
  });

  it('ignores positions while the page is hidden and re-anchors when it returns', async () => {
    const r = await open(new FakeServer());
    const binding = await readerFor(r);
    read(r, binding, 0, 600);
    r.page.emit('hidden');
    r.clock.advance(10);
    binding.observe(900);
    r.page.emit('visible');
    r.clock.advance(600);
    binding.observe(3000);
    r.clock.advance(10);
    binding.observe(3300);
    await r.engine.flush();

    expect((await storedSpans(r.name)).toPairs()).toEqual([
      [0, 600],
      [3000, 3300]
    ]);
  });

  it('records only the book that is open when its key resolves', async () => {
    const server = new FakeServer();
    const other = '犬'.repeat(5_000);
    const r = await open(server, dbName(), { books: [book(1, TEXT), book(2, other)] });
    const primed = r.engine.reader({ forwardSkipThreshold: () => 2700 });
    primed.setBook(book(2, other, 'Book 2'));
    await primed.close();
    const binding = r.engine.reader({ forwardSkipThreshold: () => 2700 });
    binding.setBook(book(1, TEXT));
    binding.setBook(book(2, other, 'Book 2'));
    read(r, binding, 0, 600);
    await r.engine.flush();

    expect(r.engine.openBook.get()).toMatchObject({ title: 'Book 2', length: 5_000 });
    const store = await openSyncStore(r.name);
    const records = await store.records();
    store.close();
    expect(records.map((record) => [record.title, record.spans.toPairs()])).toEqual([
      ['Book 2', [[0, 600]]]
    ]);
  });

  it('reports storage failure and keeps the reading for the next commit', async () => {
    const r = await open(new FakeServer());
    const binding = await readerFor(r);
    const put = IDBObjectStore.prototype.put;
    IDBObjectStore.prototype.put = function () {
      throw new DOMException('Quota exceeded', 'QuotaExceededError');
    };
    try {
      read(r, binding, 0, 600);
      await r.engine.flush();
      expect(status(r)).toMatchObject({ kind: 'storage_failed', message: 'Quota exceeded' });
    } finally {
      IDBObjectStore.prototype.put = put;
    }
    r.clock.advance(10);
    binding.observe(900);
    await r.engine.flush();
    await r.engine.drain(5_000);

    expect(status(r).kind).toBe('not_configured');
    expect((await storedSpans(r.name)).toPairs()).toEqual([[0, 900]]);
  });
});

describe('legacy v7 rows', () => {
  function v7Source() {
    return new FakeLegacy(
      [
        legacyRow('0f8fad5b-d9cb-469f-a165-70867728950e', 300, 300, 1),
        legacyRow('7c9e6679-7425-40de-944b-e07fc1f90ae7', 200, 500, 2)
      ],
      [
        {
          readerDataId: 1,
          flushedPosition: 500,
          confirmedPosition: 800,
          updatedAt: Date.parse('2026-10-21T12:00:00')
        }
      ]
    );
  }

  it('drains originals and the tail once, survives a lost reply, then claims the book', async () => {
    const server = new FakeServer();
    const r = await open(server, dbName(), { legacy: v7Source() });
    server.loseReplies = 2;
    server.loseRepliesTo = '/events';
    await configure(r);
    await r.engine.retryNow();
    await r.engine.retryNow();
    expect(server.loseReplies).toBe(0);

    expect(server.legacyTotal()).toBe(800);
    expect(server.legacyEvents.get('v7-tail-1-500-800')).toMatchObject({
      date: '2026-10-21',
      delta: 300,
      confirmed_position_after: 800
    });
    expect(server.claims.get(1)).toEqual({
      book_key: await key(),
      title: 'Book 1',
      length: 10_000
    });
    expect(status(r)).toMatchObject({ kind: 'synced', pending: { older: 0 } });

    const requests = server.requests.length;
    expect(await r.engine.drain(5_000)).toBe('idle');
    expect(server.requests.length).toBe(requests);
  });

  it('delivers every legacy row and claim again to a new server identity', async () => {
    const server = new FakeServer();
    const r = await open(server, dbName(), { legacy: v7Source() });
    await configure(r);
    expect(server.legacyTotal()).toBe(800);

    server.serverId = '5b0c1f8e-2f4d-4c43-9a43-7c1fd0b5a002';
    server.legacyEvents.clear();
    server.claims.clear();
    await r.engine.retryNow();

    expect(server.legacyTotal()).toBe(800);
    expect(server.claims.has(1)).toBe(true);
    expect(status(r)).toMatchObject({ kind: 'synced', pending: { older: 0 } });
  });

  it('holds new reports for a legacy book until its claim lands', async () => {
    const server = new FakeServer();
    const r = await open(server, dbName(), { legacy: v7Source() });
    server.answer.set('/legacy-books/', 409);
    await configure(r);
    const binding = await readerFor(r);
    read(r, binding, 0, 600);
    await r.engine.drain(5_000);
    expect(status(r)).toMatchObject({ kind: 'blocked', problem: { httpStatus: 409 } });
    expect(server.requests.some((request) => request.includes('/days/'))).toBe(false);

    server.answer.clear();
    await r.engine.retryNow();
    expect(server.claims.has(1)).toBe(true);
    expect(server.credited(await key())).toBe(600);
  });

  it('retries a rejected claim after 6 hours without Retry now', async () => {
    const server = new FakeServer();
    const r = await open(server, dbName(), { legacy: v7Source() });
    server.answer.set('/legacy-books/', 409);
    await configure(r);
    const binding = await readerFor(r);
    read(r, binding, 0, 600);
    await r.engine.drain(5_000);
    expect(server.claims.has(1)).toBe(false);

    server.answer.clear();
    r.clock.advance(6 * 3600);
    await r.engine.drain(5_000);
    expect(server.claims.has(1)).toBe(true);
    expect(server.credited(await key())).toBe(600);
  });

  it('keeps every row a failing server did not accept, and sends it later', async () => {
    const server = new FakeServer();
    const r = await open(server, dbName(), { legacy: v7Source() });
    server.answer.set('/events', 500);
    await configure(r);
    expect(server.legacyTotal()).toBe(0);
    expect(status(r)).toMatchObject({ kind: 'waiting', pending: { older: 4 } });

    server.answer.clear();
    await r.engine.retryNow();
    expect(server.legacyTotal()).toBe(800);
  });

  it('holds only the book of a rejected row and delivers everything else', async () => {
    const server = new FakeServer();
    const other = '犬'.repeat(5_000);
    const legacy = v7Source();
    legacy.queuedRows.push({
      ...legacyRow('b0b0b0b0-0000-4000-8000-000000000002', 100, 100, 3, 2),
      title: ' '
    });
    legacy.library.set(2, { title: 'Book 2', characters: 5_000 });
    const r = await open(server, dbName(), {
      legacy,
      books: [book(1, TEXT), book(2, other, 'Book 2')]
    });
    await configure(r);
    const first = await readerFor(r);
    read(r, first, 0, 600);
    await first.close();
    const second = r.engine.reader({ forwardSkipThreshold: () => 2700 });
    second.setBook(book(2, other, 'Book 2'));
    await r.engine.flush();
    read(r, second, 0, 900);
    await r.engine.drain(5_000);

    expect(server.legacyTotal()).toBe(800);
    expect(server.claims.has(1)).toBe(true);
    expect(server.credited(await key())).toBe(600);
    expect(server.claims.has(2)).toBe(false);
    const otherKey = (await bookIdentity(other))?.key ?? '';
    expect(server.credited(otherKey)).toBe(0);
    expect(status(r)).toMatchObject({
      kind: 'blocked',
      blockedBooks: [{ title: 'Book 2', problem: { kind: 'rejected', httpStatus: 400 } }]
    });
  });

  it('sends a stored target of 0 as a positive target', async () => {
    const server = new FakeServer();
    const legacy = v7Source();
    legacy.queuedRows[0] = { ...legacy.queuedRows[0], targetCharacters: 0 };
    const r = await open(server, dbName(), { legacy });
    await configure(r);

    expect(server.legacyEvents.get(legacy.queuedRows[0].eventId)).toMatchObject({
      book_target_characters: 300
    });
    expect(status(r).kind).toBe('synced');
  });

  it('reports a legacy book that left the library and still drains its rows', async () => {
    const server = new FakeServer();
    const r = await open(server, dbName(), { legacy: v7Source(), books: [] });
    await configure(r);

    expect(server.legacyTotal()).toBe(800);
    expect(status(r)).toMatchObject({
      kind: 'blocked',
      blockedBooks: [{ title: 'Book 1', problem: { kind: 'not_in_library', readerDataId: 1 } }]
    });
  });
});

describe('host drain hook', () => {
  it('times out instead of hanging on a stalled server', async () => {
    const stalled = Object.assign(new FakeServer(), {
      fetch: () => new Promise<Response>(() => undefined)
    });
    const r = await open(stalled, dbName(), { timers: realTimers });
    await r.engine.configure(BASE);
    expect(await r.engine.drain(50)).toBe('timeout');
  });
});
