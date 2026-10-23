/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { describe, expect, it } from 'vitest';

import { bookIdentity } from '../domain/book-key';
import type { DayRecord } from '../domain/day-record';
import { ready } from '../domain/delivery-policy';
import { newProducerId, readingDayOf, type BookKey, type ReadingDay } from '../domain/ids';
import { parseServerUrl, type ServerUrl } from '../domain/server-url';
import { SpanSet } from '../domain/span-set';
import { Clock, FakeLegacy, book, legacyRow, realTimers, rig, started } from '../test/harness';
import {
  describeServer,
  postLegacyEvent,
  provisionLegacy,
  putDay,
  putLegacyBook,
  type LegacyEvent
} from './http';

/** Runs only against an isolated server: pnpm test:contract starts none, it expects one. */
const raw = process.env.LIFESTATS_CONTRACT_URL;

function url(value: string): ServerUrl {
  const parsed = parseServerUrl(value);
  if (!parsed.ok) throw new Error(`bad url ${value}`);
  return parsed.url;
}

async function uniqueBook(label: string) {
  const identity = await bookIdentity(
    `${label}${crypto.randomUUID()}`.replace(/[^0-9a-z]/gi, '') + '猫'.repeat(10_000)
  );
  if (!identity) throw new Error('no identity');
  return { ...identity, text: label };
}

function record(
  key: BookKey,
  length: number,
  spans: [number, number][],
  day: ReadingDay
): DayRecord {
  const set = SpanSet.fromPairs(spans) ?? SpanSet.empty;
  return {
    book: key,
    day,
    title: `契約テスト ${key.slice(-8)}`,
    length,
    spans: set,
    lastPosition: set.end,
    observedAt: Date.now(),
    revision: 1,
    delivered: null,
    delivery: ready
  };
}

async function statisticId(base: ServerUrl): Promise<number> {
  const statistics: { id: number; name: string }[] = await (
    await fetch(`${base}/statistics`)
  ).json();
  const found = statistics.find((statistic) => statistic.name === '[Ebook] Characters Read');
  if (!found) throw new Error('no ebook statistic');
  return found.id;
}

async function logged(base: ServerUrl, day: string): Promise<number> {
  const id = await statisticId(base);
  const body: { value?: number } = await (
    await fetch(`${base}/statistics/${id}/logged/${day}`)
  ).json();
  return body.value ?? 0;
}

describe.skipIf(!raw)('contract with an isolated Lifestats server', () => {
  const base = url(raw ?? 'http://127.0.0.1:1');
  const today = readingDayOf(new Date());
  const producer = newProducerId();

  it('describes itself with a server id and protocol 1', async () => {
    const description = await describeServer(fetch, base);
    expect(description).toMatchObject({ kind: 'ok' });
    expect(await describeServer(fetch, base)).toEqual(description);
  });

  it('accepts every day report shape the client produces', async () => {
    const { key, length } = await uniqueBook('shapes');
    for (const spans of [
      [[0, 300]],
      [],
      [
        [0, 100],
        [200, length]
      ]
    ] as [number, number][][]) {
      expect(await putDay(fetch, base, producer, record(key, length, spans, today))).toEqual({
        kind: 'delivered'
      });
    }
    const yesterday = readingDayOf(new Date(Date.now() - 86_400_000));
    expect(await putDay(fetch, base, producer, record(key, length, [[0, 50]], yesterday))).toEqual({
      kind: 'delivered'
    });
  });

  it('accepts legacy provisioning, events and duplicate events', async () => {
    const event: LegacyEvent = {
      event_id: crypto.randomUUID(),
      date: today,
      metric: 'characters_read',
      delta: 7,
      book_id: 900_000 + Math.floor(Math.random() * 99_999),
      book_title: `Legacy contract ${crypto.randomUUID()}`,
      confirmed_position_after: 7,
      book_target_characters: 100
    };
    expect(await provisionLegacy(fetch, base)).toEqual({ kind: 'delivered' });
    expect(await postLegacyEvent(fetch, base, event)).toEqual({ kind: 'delivered' });
    expect(await postLegacyEvent(fetch, base, event)).toEqual({ kind: 'delivered' });
  });

  it('maps each error class to the outcome the client acts on', async () => {
    const { key, length } = await uniqueBook('errors');
    await putDay(fetch, base, producer, record(key, length, [[0, 10]], today));

    expect(
      await putDay(fetch, base, producer, record(key, length - 1, [[0, 10]], today))
    ).toMatchObject({
      kind: 'needs_fix',
      problem: { kind: 'rejected', httpStatus: 409, code: 'book_length_mismatch' }
    });
    const future = readingDayOf(new Date(Date.now() + 10 * 86_400_000));
    expect(
      await putDay(fetch, base, producer, record(key, length, [[0, 10]], future))
    ).toMatchObject({
      kind: 'needs_fix',
      problem: { kind: 'rejected', httpStatus: 422, code: 'invalid_field', field: 'day' }
    });

    const title = `Legacy adoption ${crypto.randomUUID()}`;
    const readerDataId = 800_000 + Math.floor(Math.random() * 99_999);
    await postLegacyEvent(fetch, base, {
      event_id: crypto.randomUUID(),
      date: today,
      metric: 'characters_read',
      delta: 5,
      book_id: readerDataId,
      book_title: title,
      confirmed_position_after: 5,
      book_target_characters: 100
    });
    const adopted = await uniqueBook('adopted');
    expect(
      await putDay(fetch, base, producer, {
        ...record(adopted.key, adopted.length, [[0, 10]], today),
        title
      })
    ).toMatchObject({
      kind: 'needs_fix',
      problem: { httpStatus: 409, code: 'legacy_adoption_pending' }
    });
    const claim = { book: adopted.key, title, length: adopted.length };
    expect(await putLegacyBook(fetch, base, readerDataId, claim)).toEqual({ kind: 'delivered' });
    expect(await putLegacyBook(fetch, base, readerDataId, claim)).toEqual({ kind: 'delivered' });
    const other = await uniqueBook('other');
    expect(
      await putLegacyBook(fetch, base, readerDataId, {
        ...claim,
        book: other.key,
        length: other.length
      })
    ).toMatchObject({
      kind: 'needs_fix',
      problem: { httpStatus: 409, code: 'legacy_binding_conflict' }
    });
    expect(
      await putDay(fetch, base, producer, {
        ...record(adopted.key, adopted.length, [[0, 10]], today),
        title
      })
    ).toEqual({ kind: 'delivered' });

    const notLifestats = url(`${base}/no-such-prefix`);
    expect(await describeServer(fetch, notLifestats)).toMatchObject({
      kind: 'needs_fix',
      problem: { kind: 'not_lifestats' }
    });
    expect(
      await putDay(fetch, notLifestats, producer, record(key, length, [], today))
    ).toMatchObject({
      kind: 'needs_fix',
      problem: { kind: 'server_too_old' }
    });
    expect(await describeServer(fetch, url('http://127.0.0.1:1'))).toMatchObject({
      kind: 'retryable',
      problem: { kind: 'unreachable' }
    });
  });

  it('counts a drained v7 event once when its 200 was lost and it is sent again', async () => {
    const day = readingDayOf(new Date(Date.now() - 3 * 86_400_000));
    const before = await logged(base, day);
    const readerDataId = 700_000 + Math.floor(Math.random() * 99_999);
    const text = `drain${crypto.randomUUID()}`.replace(/[^0-9a-z]/gi, '') + '猫'.repeat(5_000);
    const title = `Drain contract ${readerDataId}`;
    const legacy = new FakeLegacy(
      [{ ...legacyRow(crypto.randomUUID(), 400, 400, 1, readerDataId), day, title }],
      [{ readerDataId, flushedPosition: 400, confirmedPosition: 650, updatedAt: Date.now() }],
      new Map([[readerDataId, { title, characters: 5_000 }]])
    );
    legacy.dayOf = () => day;

    let lose = 1;
    const lossy: typeof fetch = async (input, init) => {
      const response = await fetch(input, init);
      if (String(input).endsWith('/events') && lose > 0) {
        lose -= 1;
        throw new TypeError('reply lost');
      }
      return response;
    };
    const r = rig({
      server: { fetch: lossy },
      dbName: `ttu-lifestats-contract-${crypto.randomUUID()}`,
      books: [book(readerDataId, text, title)],
      legacy,
      timers: realTimers,
      clock: new Clock(Date.now())
    });
    await started(r.engine);
    try {
      await r.engine.configure(base);
      await r.engine.drain(10_000);
      await r.engine.retryNow();
      await r.engine.retryNow();
      expect(lose).toBe(0);
      expect(await logged(base, day)).toBe(before + 400 + 250);

      const binding = r.engine.reader({ forwardSkipThreshold: () => 2700 });
      binding.setBook(book(readerDataId, text, title));
      await r.engine.flush();
      const todayBefore = await logged(base, today);
      r.engine.relocate();
      binding.observe(650);
      r.clock.advance(60);
      binding.observe(900);
      expect(await r.engine.drain(10_000)).toBe('idle');
      expect(await logged(base, today)).toBe(todayBefore + 250);
      expect(await r.engine.drain(10_000)).toBe('idle');
      expect(await logged(base, today)).toBe(todayBefore + 250);
    } finally {
      r.engine.close();
    }
  });
});
