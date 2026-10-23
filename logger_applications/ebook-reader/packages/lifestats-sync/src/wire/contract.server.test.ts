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
import { describeServer, putDay } from './http';

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

async function logged(base: ServerUrl, day: string): Promise<number> {
  const statistics: { id: number; name: string }[] = await (
    await fetch(`${base}/statistics`)
  ).json();
  const found = statistics.find((statistic) => statistic.name === '[Ebook] Characters Read');
  if (!found) throw new Error('no ebook statistic');
  const body: { value?: number } = await (
    await fetch(`${base}/statistics/${found.id}/logged/${day}`)
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

    expect(
      await putDay(
        fetch,
        base,
        producer,
        record(key, length, [[0, 10]], '2000-01-01' as ReadingDay)
      )
    ).toMatchObject({
      kind: 'needs_fix',
      problem: { httpStatus: 422, field: 'day' }
    });

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
  it('returns upgrade envelopes from retired POSTs without changing the statistic', async () => {
    const before = await logged(base, today);
    for (const path of ['/integrations/ebook', '/integrations/ebook/events']) {
      const response = await fetch(`${base}${path}`, { method: 'POST', body: '{}' });
      expect(response.status).toBe(410);
      expect(await response.json()).toMatchObject({ error: { code: 'client_upgrade_required' } });
    }
    expect(await logged(base, today)).toBe(before);
  });

  it('credits a day once after its successful reply is lost', async () => {
    const { key, length } = await uniqueBook('lost-reply');
    const report = record(key, length, [[0, 400]], today);
    const before = await logged(base, today);
    const lossy: typeof fetch = async (input, init) => {
      const response = await fetch(input, init);
      expect(response.status).toBe(204);
      throw new TypeError('reply lost');
    };
    expect(await putDay(lossy, base, producer, report)).toMatchObject({ kind: 'retryable' });
    expect(await logged(base, today)).toBe(before + 400);
    expect(await putDay(fetch, base, producer, report)).toEqual({ kind: 'delivered' });
    expect(await logged(base, today)).toBe(before + 400);
  });
});
