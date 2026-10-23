/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import 'fake-indexeddb/auto';

import type { LegacyQueued, LegacySource, LegacyTail, LocalBook, SyncInit } from '../api';
import { SpanSet } from '../domain/span-set';
import { Engine, type EngineEnvironment, type PageEvent, type Timers } from '../engine/engine';

export const BASE = 'http://lifestats.test';

/** In-memory stand-in for the server contract in docs/ebook-reading/api.md. */
export class FakeServer {
  serverId = '5b0c1f8e-2f4d-4c43-9a43-7c1fd0b5a001';
  down = false;
  /** Path substring to status code, answered instead of handling the request. */
  answer = new Map<string, number>();
  /** Handle this many writes to `loseRepliesTo`, then fail each as if its reply was lost. */
  loseReplies = 0;
  loseRepliesTo = '/integrations/ebook/';
  readonly days = new Map<string, SpanSet>();
  readonly lengths = new Map<string, number>();
  readonly legacyEvents = new Map<string, Record<string, unknown>>();
  readonly claims = new Map<number, Record<string, unknown>>();
  readonly requests: string[] = [];

  /** Characters credited across all producers and days. */
  credited(book: string): number {
    let union = SpanSet.empty;
    for (const [key, spans] of this.days)
      if (key.split('|')[1] === book) union = union.union(spans);
    return union.size;
  }

  legacyTotal(): number {
    return [...this.legacyEvents.values()].reduce((total, event) => total + Number(event.delta), 0);
  }

  readonly fetch: typeof fetch = async (input, init) => {
    const url = new URL(String(input));
    const method = init?.method ?? 'GET';
    this.requests.push(`${method} ${url.pathname}`);
    if (this.down) throw new TypeError('Failed to fetch');
    for (const [fragment, status] of this.answer) {
      if (url.pathname.includes(fragment)) return json(status, envelope(status));
    }
    const response = this.handle(
      method,
      url.pathname,
      init?.body ? JSON.parse(String(init.body)) : undefined
    );
    if (method !== 'GET' && url.pathname.includes(this.loseRepliesTo) && this.loseReplies > 0) {
      this.loseReplies -= 1;
      throw new TypeError('Failed to fetch');
    }
    return response;
  };

  private handle(
    method: string,
    path: string,
    body: Record<string, unknown> | undefined
  ): Response {
    if (method === 'GET' && path === '/integrations/ebook') {
      return json(200, { server_id: this.serverId, protocol: 1 });
    }
    if (method === 'POST' && path === '/integrations/ebook') return json(200, { ebook: 1 });
    if (method === 'POST' && path === '/integrations/ebook/events' && body) {
      if (String(body.book_title).trim() === '' || Number(body.book_target_characters) <= 0) {
        return json(400, { error: { code: 'bad_request', field: null, message: 'Invalid event' } });
      }
      const id = String(body.event_id);
      const duplicate = this.legacyEvents.has(id);
      if (!duplicate) this.legacyEvents.set(id, body);
      return json(200, { accepted: !duplicate, duplicate });
    }
    const claim = /^\/integrations\/ebook\/legacy-books\/(\d+)$/.exec(path);
    if (method === 'PUT' && claim && body) {
      this.claims.set(Number(claim[1]), body);
      return new Response(null, { status: 204 });
    }
    const day = /^\/integrations\/ebook\/producers\/([^/]+)\/books\/([^/]+)\/days\/([^/]+)$/.exec(
      path
    );
    if (method === 'PUT' && day && body) {
      const [, producer, rawBook, date] = day;
      const book = decodeURIComponent(rawBook);
      const spans = SpanSet.fromPairs(body.spans);
      const length = Number(body.length);
      const pairs = JSON.stringify(body.spans);
      if (!spans || JSON.stringify(spans.toPairs()) !== pairs || spans.end > length) {
        return json(422, {
          error: { code: 'invalid_field', field: 'spans', message: 'Invalid spans' }
        });
      }
      if (this.lengths.has(book) && this.lengths.get(book) !== length) {
        return json(409, {
          error: { code: 'book_length_mismatch', field: 'length', message: 'x' }
        });
      }
      this.lengths.set(book, length);
      const key = `${producer}|${book}|${date}`;
      this.days.set(key, (this.days.get(key) ?? SpanSet.empty).union(spans));
      return new Response(null, { status: 204 });
    }
    return json(404, envelope(404));
  }
}

function envelope(status: number) {
  return {
    error: { code: status === 422 ? 'invalid_field' : `code_${status}`, field: null, message: 'm' }
  };
}

function json(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' }
  });
}

/** The v7 stores of a laptop profile, with book 1 still in its library. */
export class FakeLegacy implements LegacySource {
  constructor(
    public queuedRows: LegacyQueued[],
    public tailRows: LegacyTail[],
    public library = new Map([[1, { title: 'Book 1', characters: 10_000 }]])
  ) {}

  queued = async () => this.queuedRows;
  tails = async () => this.tailRows;
  book = async (readerDataId: number) => this.library.get(readerDataId);
  dayOf = () => '2026-10-21';
}

export function legacyRow(
  eventId: string,
  delta: number,
  positionAfter: number,
  createdAt: number,
  readerDataId = 1
): LegacyQueued {
  return {
    eventId,
    readerDataId,
    day: '2026-10-20',
    delta,
    positionAfter,
    targetCharacters: 10_000,
    title: `Book ${readerDataId}`,
    createdAt
  };
}

/** Background work never fires on its own, so each test decides when delivery runs. */
const manualTimers: Timers = { set: () => undefined, clear: () => undefined };

export const realTimers: Timers = {
  set: (task, delayMs) => setTimeout(task, delayMs),
  clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>)
};

export class FakePage {
  private hiddenNow = false;
  private readonly listeners = new Set<(event: PageEvent) => void>();

  hidden = () => this.hiddenNow;

  listen = (listener: (event: PageEvent) => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  emit(event: PageEvent): void {
    if (event === 'hidden' || event === 'visible') this.hiddenNow = event === 'hidden';
    this.listeners.forEach((listener) => listener(event));
  }
}

export class Clock {
  constructor(public ms = Date.parse('2026-10-22T10:00:00')) {}
  now = () => this.ms;
  advance(seconds: number): void {
    this.ms += seconds * 1000;
  }
}

export function book(localId: number, text: string, title = `Book ${localId}`): LocalBook {
  return { localId, title, modified: 1, countedText: () => text };
}

export interface Rig {
  readonly engine: Engine;
  readonly clock: Clock;
  readonly page: FakePage;
}

export function rig(options: {
  server: { fetch: typeof fetch };
  dbName: string;
  books?: LocalBook[];
  legacy?: SyncInit['legacy'];
  buildId?: string;
  clock?: Clock;
  timers?: Timers;
}): Rig {
  const clock = options.clock ?? new Clock();
  const page = new FakePage();
  const library = new Map((options.books ?? []).map((local) => [local.localId, local]));
  const env: EngineEnvironment = {
    fetch: options.server.fetch,
    now: clock.now,
    random: () => 0.5,
    hidden: page.hidden,
    listen: page.listen,
    locks: null,
    idle: (task) => task(),
    timers: options.timers ?? manualTimers,
    dbName: options.dbName
  };
  const engine = new Engine(
    {
      buildId: options.buildId ?? 'build-1',
      library: { book: async (localId) => library.get(localId) },
      legacy: options.legacy ?? null
    },
    env
  );
  return { engine, clock, page };
}

/** Resolves once the engine published a status other than `starting`. */
export function started(engine: Engine): Promise<void> {
  return new Promise((resolve) => {
    const stop = engine.status.subscribe((status) => {
      if (status.kind !== 'starting') {
        queueMicrotask(() => stop());
        resolve();
      }
    });
  });
}
