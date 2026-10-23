/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type {
  BlockedBook,
  ConfigureResult,
  ConnectionCheck,
  LifestatsSync,
  LocalBook,
  OpenBook,
  Readable,
  ReaderBinding,
  ReaderOptions,
  SyncInit,
  SyncStatus
} from '../api';
import { bookIdentity, type BookIdentity } from '../domain/book-key';
import { isPending, type DayObservation } from '../domain/day-record';
import {
  PARKED_RETRY_MS,
  isDue,
  nextDeliveryState,
  ready,
  type DeliveryContext,
  type DeliveryState,
  type FailedAttempt
} from '../domain/delivery-policy';
import { readingDayOf, type BookKey, type ProducerId } from '../domain/ids';
import { untrackable } from '../domain/problem';
import { emptyTrail, observe, release, relocate, type Trail } from '../domain/reading-policy';
import { parseServerUrl, type ServerUrl } from '../domain/server-url';
import { SpanSet } from '../domain/span-set';
import { deriveStatus } from '../domain/status';
import { LegacyCutover } from '../legacy/cutover';
import { openSyncStore, type SyncStore } from '../store/sync-db';
import { describeServer, putDay } from '../wire/http';

/** How long after `relocate()` positions only move the anchor. Covers layout and scroll restore. */
export const SETTLE_MS = 750;
export const DELIVERY_DELAY_MS = 1_000;
export const IDLE_PASS_MS = 30_000;
export const STORAGE_RETRY_MS = 5_000;
const DEFAULT_SKIP_THRESHOLD = 2700;
const DELIVERY_LOCK = 'ttu-lifestats-delivery';

export type PageEvent = 'hidden' | 'visible' | 'online';

export interface EngineEnvironment {
  readonly fetch: typeof fetch;
  readonly now: () => number;
  readonly random: () => number;
  readonly hidden: () => boolean;
  readonly listen: (listener: (event: PageEvent) => void) => () => void;
  readonly locks: Pick<LockManager, 'request'> | null;
  readonly idle: (task: () => void) => void;
  readonly timers: Timers;
  readonly dbName: string;
}

export interface Timers {
  set(task: () => void, delayMs: number): unknown;
  clear(handle: unknown): void;
}

interface Writable<T> extends Readable<T> {
  get(): T;
  set(value: T): void;
}

function writable<T>(initial: T): Writable<T> {
  let value = initial;
  const runs = new Set<(value: T) => void>();
  return {
    get: () => value,
    set(next) {
      value = next;
      runs.forEach((run) => run(value));
    },
    subscribe(run) {
      runs.add(run);
      run(value);
      return () => runs.delete(run);
    }
  };
}

function merge(a: DayObservation, b: DayObservation): DayObservation {
  const later = b.atMs >= a.atMs ? b : a;
  return { ...later, spans: a.spans.union(b.spans) };
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

interface CutoverFacts {
  readonly older: number;
  readonly blockedBooks: readonly BlockedBook[];
}

export class Engine implements LifestatsSync {
  readonly status = writable<SyncStatus>({ kind: 'starting' });
  readonly serverUrl = writable<string | null>(null);
  readonly openBook = writable<OpenBook | null>(null);

  private readonly opened: Promise<{ store: SyncStore; producer: ProducerId }>;
  private readonly buffer = new Map<string, DayObservation>();
  private readonly bindings = new Set<Binding>();
  private readonly identities = new Map<string, Promise<BookIdentity | undefined>>();
  /** Open books whose key failed to resolve, by local id. */
  private readonly untrackable = new Map<number, BlockedBook>();
  /** Book keys still resolving for open readers; their positions wait in the reader. */
  private readonly resolving = new Set<Promise<unknown>>();
  private readonly disposers: (() => void)[] = [];
  private committing: Promise<void> | null = null;
  private storageError: string | null = null;
  private server: DeliveryState = ready;
  private describeNext = true;
  private passing: Promise<void> | null = null;
  private passAgain = false;
  private timer: unknown = null;
  private timerAt = Infinity;
  private refreshing: unknown = null;
  private settleUntil = 0;
  private readonly cutover: LegacyCutover | null;
  private cutoverFacts: CutoverFacts = { older: 0, blockedBooks: [] };
  private closed = false;

  constructor(
    private readonly init: SyncInit,
    private readonly env: EngineEnvironment
  ) {
    this.cutover =
      init.legacy &&
      new LegacyCutover(
        init.legacy,
        init.library,
        (book) => this.identityFor(book),
        `${env.dbName}-legacy`
      );
    this.opened = openSyncStore(env.dbName).then(async (store) => ({
      store,
      producer: await store.producer()
    }));
    this.opened
      .then(async ({ store }) => {
        this.serverUrl.set((await store.config()).serverUrl);
        this.disposers.push(store.onChange(() => this.refreshSoon()));
        await this.refresh();
        this.schedule(0);
      })
      .catch((error) => this.storageFailed(error));
    this.disposers.push(env.listen((event) => this.onPage(event)));
    const tick = () => {
      this.schedule(0);
      idle = env.timers.set(tick, IDLE_PASS_MS);
    };
    let idle = env.timers.set(tick, IDLE_PASS_MS);
    this.disposers.push(() => env.timers.clear(idle));
  }

  reader(options: ReaderOptions): ReaderBinding {
    const binding = new Binding(this, options);
    this.bindings.add(binding);
    return binding;
  }

  relocate(): void {
    this.settleUntil = this.env.now() + SETTLE_MS;
  }

  async configure(raw: string | null): Promise<ConfigureResult> {
    let url: ServerUrl | null = null;
    if (raw !== null && raw.trim() !== '') {
      const parsed = parseServerUrl(raw);
      if (!parsed.ok) return parsed;
      url = parsed.url;
    }
    const { store } = await this.opened;
    await store.setServerUrl(url);
    this.serverUrl.set(url);
    this.server = ready;
    this.describeNext = true;
    await this.refresh();
    void this.deliver();
    return { ok: true, serverUrl: url };
  }

  async testConnection(raw: string): Promise<ConnectionCheck> {
    const parsed = parseServerUrl(raw);
    if (!parsed.ok) return parsed;
    const description = await describeServer(this.env.fetch, parsed.url);
    return description.kind === 'ok'
      ? { kind: 'ok', serverId: description.serverId }
      : { kind: 'problem', problem: description.problem };
  }

  async retryNow(): Promise<void> {
    const { store } = await this.opened;
    await store.rearm();
    await this.cutover?.rearm();
    this.bindings.forEach((binding) => binding.retry());
    await this.flush();
    this.server = ready;
    this.describeNext = true;
    await this.deliver();
  }

  async drain(timeoutMs: number): Promise<'idle' | 'pending' | 'timeout'> {
    const work = (async () => {
      await this.flush();
      await this.deliver();
      return this.status.get().kind === 'synced' ? ('idle' as const) : ('pending' as const);
    })().catch(() => 'pending' as const);
    let timer: unknown;
    const timeout = new Promise<'timeout'>((resolve) => {
      timer = this.env.timers.set(() => resolve('timeout'), timeoutMs);
    });
    try {
      return await Promise.race([work, timeout]);
    } finally {
      this.env.timers.clear(timer);
    }
  }

  /** Waits until every observation so far is committed or storage has failed. */
  async flush(): Promise<void> {
    while (this.resolving.size) await Promise.all(this.resolving);
    while (this.committing) await this.committing;
  }

  track(resolution: Promise<unknown>): void {
    this.resolving.add(resolution);
    void resolution.finally(() => this.resolving.delete(resolution));
  }

  close(): void {
    this.closed = true;
    this.disposers.splice(0).forEach((dispose) => dispose());
    this.env.timers.clear(this.timer);
    this.env.timers.clear(this.refreshing);
    this.cutover?.close();
    void this.opened.then(({ store }) => store.close()).catch(() => undefined);
  }

  hidden(): boolean {
    return this.env.hidden();
  }

  now(): number {
    return this.env.now();
  }

  settling(atMs: number): boolean {
    return atMs < this.settleUntil;
  }

  unbind(binding: Binding): void {
    this.bindings.delete(binding);
  }

  record(observation: DayObservation): void {
    const key = `${observation.book}\n${observation.day}`;
    const existing = this.buffer.get(key);
    this.buffer.set(key, existing ? merge(existing, observation) : observation);
    this.commitSoon();
  }

  identityFor(book: LocalBook): Promise<BookIdentity | undefined> {
    const key = `${book.localId}:${book.modified}`;
    let identity = this.identities.get(key);
    if (!identity) {
      identity = this.resolveIdentity(book);
      this.identities.set(key, identity);
      identity.catch(() => this.identities.delete(key));
    }
    return identity;
  }

  /** Whether `book`'s key resolved, for the status. */
  resolved(book: LocalBook, error: unknown | null): void {
    if (error === null) this.untrackable.delete(book.localId);
    else this.untrackable.set(book.localId, { title: book.title, problem: untrackable(error) });
    this.refreshSoon();
  }

  private async resolveIdentity(book: LocalBook): Promise<BookIdentity | undefined> {
    const { store } = await this.opened;
    const cached = await store.identity(book.localId, book.modified);
    if (cached) return cached;
    const text = await new Promise<string>((resolve, reject) =>
      this.env.idle(() => {
        try {
          resolve(book.countedText());
        } catch (error) {
          reject(error);
        }
      })
    );
    const identity = await bookIdentity(text);
    // A failed cache write costs only a recomputation next time.
    if (identity) await store.putIdentity(book.localId, book.modified, identity).catch(() => {});
    return identity;
  }

  private onPage(event: PageEvent): void {
    if (event === 'hidden') {
      this.schedule(0);
    } else if (event === 'visible') {
      this.bindings.forEach((binding) => binding.release());
      this.schedule(DELIVERY_DELAY_MS);
    } else {
      this.describeNext = true;
      this.schedule(0);
    }
  }

  /** Write-behind: at most one commit in flight, later observations coalesce into the next. */
  private commitSoon(): void {
    if (this.committing || this.closed) return;
    this.committing = this.commitLoop().finally(() => {
      this.committing = null;
      if (this.buffer.size && !this.storageError) this.commitSoon();
    });
  }

  private async commitLoop(): Promise<void> {
    while (this.buffer.size) {
      const batch = [...this.buffer.values()];
      this.buffer.clear();
      try {
        const { store } = await this.opened;
        await store.commit(batch);
        this.storageError = null;
      } catch (error) {
        batch.forEach((observation) => this.record(observation));
        this.storageFailed(error);
        this.env.timers.set(() => {
          this.storageError = null;
          this.commitSoon();
        }, STORAGE_RETRY_MS);
        return;
      }
    }
    this.refreshSoon();
    this.schedule(DELIVERY_DELAY_MS);
  }

  private storageFailed(error: unknown): void {
    this.storageError = message(error);
    this.status.set({
      kind: 'storage_failed',
      message: this.storageError,
      pending: { records: 0, characters: 0, oldestDay: null, older: 0 },
      blockedBooks: [],
      lastDeliveredAt: null
    });
  }

  private schedule(delayMs: number): void {
    if (this.closed) return;
    const at = this.env.now() + delayMs;
    if (this.timer && this.timerAt <= at) return;
    this.env.timers.clear(this.timer);
    this.timerAt = at;
    this.timer = this.env.timers.set(() => {
      this.timer = null;
      this.timerAt = Infinity;
      void this.deliver();
    }, delayMs);
  }

  deliver(): Promise<void> {
    if (this.passing) {
      this.passAgain = true;
      return this.passing;
    }
    this.passing = (async () => {
      do {
        this.passAgain = false;
        await this.lockedPass().catch((error) => this.storageFailed(error));
      } while (this.passAgain && !this.closed);
    })().finally(() => {
      this.passing = null;
    });
    return this.passing;
  }

  private lockedPass(): Promise<void> {
    const { locks } = this.env;
    if (!locks) return this.pass();
    return locks.request(DELIVERY_LOCK, { ifAvailable: true }, async (lock) => {
      if (lock) await this.pass();
      else this.schedule(DELIVERY_DELAY_MS);
    });
  }

  private async pass(): Promise<void> {
    await this.flush();
    if (this.storageError || this.closed) return;
    const { store, producer } = await this.opened;
    const config = await store.config();
    const base = config.serverUrl;
    const context: DeliveryContext = {
      now: this.env.now(),
      buildId: this.init.buildId,
      configRevision: config.revision
    };
    const older = (await this.cutover?.pending(config.serverId)) ?? 0;
    this.cutoverFacts = { ...this.cutoverFacts, older };

    const records = await store.records();
    const work = older > 0 || records.some((record) => isPending(record, config.serverId));
    if (!base || (!work && !this.describeNext)) return this.refresh();
    if (!isDue(this.server, context)) {
      this.retryServerLater(context);
      return this.refresh();
    }

    const description = await describeServer(this.env.fetch, base);
    if (description.kind !== 'ok') return this.serverFailed(description, context);
    this.describeNext = false;
    const { serverId } = description;
    if (serverId !== config.serverId) await store.setServerId(serverId);

    let held: ReadonlySet<BookKey> = new Set();
    if (this.cutover) {
      const report = await this.cutover.run(
        { fetch: this.env.fetch, base, serverId, context, random: this.env.random },
        (book) => store.rearm((record) => record.book === book)
      );
      if ('kind' in report) return this.serverFailed(report, context);
      held = report.held;
      this.cutoverFacts = { older: report.pending, blockedBooks: report.blocked };
    }

    const due = (await store.records())
      .filter(
        (record) =>
          isPending(record, serverId) && isDue(record.delivery, context) && !held.has(record.book)
      )
      .sort((a, b) => a.day.localeCompare(b.day) || a.book.localeCompare(b.book));
    for (const record of due) {
      const outcome = await putDay(this.env.fetch, base, producer, record);
      if (outcome.kind === 'delivered') {
        await store.acknowledge(record, serverId, this.env.now());
        continue;
      }
      if (outcome.kind === 'retryable') return this.serverFailed(outcome, context);
      await store.fail(
        record,
        nextDeliveryState(record.delivery, outcome, context, this.env.random)
      );
    }
    this.server = ready;
    await this.refresh();
  }

  private async serverFailed(outcome: FailedAttempt, context: DeliveryContext): Promise<void> {
    this.server = nextDeliveryState(this.server, outcome, context, this.env.random);
    this.retryServerLater(context);
    await this.refresh();
  }

  private retryServerLater(context: DeliveryContext): void {
    if (this.server.kind === 'retrying') {
      this.schedule(Math.max(0, this.server.nextAttemptAt - context.now));
    } else if (this.server.kind === 'parked') {
      this.schedule(PARKED_RETRY_MS);
    }
  }

  private refreshSoon(): void {
    if (this.refreshing || this.closed) return;
    this.refreshing = this.env.timers.set(() => {
      this.refreshing = null;
      void this.refresh().catch((error) => this.storageFailed(error));
    }, 100);
  }

  private async refresh(): Promise<void> {
    const { store } = await this.opened;
    const [config, records, lastDeliveredAt] = await Promise.all([
      store.config(),
      store.records(),
      store.lastDeliveredAt()
    ]);
    this.status.set(
      deriveStatus({
        configured: config.serverUrl !== null,
        storageError: this.storageError,
        records,
        serverId: config.serverId,
        server: this.server,
        lastDeliveredAt,
        older: this.cutoverFacts.older,
        blockedBooks: [...this.cutoverFacts.blockedBooks, ...this.untrackable.values()]
      })
    );
  }
}

interface CurrentBook {
  readonly local: LocalBook;
  /** `undefined` while resolving or after a failure; `null` for a book with no countable text. */
  identity: BookIdentity | null | undefined;
  failed: boolean;
}

class Binding implements ReaderBinding {
  private trail: Trail = emptyTrail;
  private book: CurrentBook | null = null;
  private unkeyed: { spans: SpanSet; position: number; atMs: number }[] = [];
  /** The reader reports a position only when it changes, often before the book is known. */
  private lastPosition: number | null = null;

  constructor(
    private readonly engine: Engine,
    private readonly options: ReaderOptions
  ) {}

  setBook(local: LocalBook | null): void {
    if (
      local?.localId === this.book?.local.localId &&
      local?.modified === this.book?.local.modified
    ) {
      return;
    }
    this.trail =
      this.lastPosition === null
        ? emptyTrail
        : relocate(emptyTrail, this.lastPosition, this.engine.now(), this.policy()).trail;
    this.unkeyed = [];
    this.engine.openBook.set(null);
    this.book = local && { local, identity: undefined, failed: false };
    if (this.book) this.resolve(this.book);
  }

  /** Retry now: resolves a key that failed, keeping the reading observed meanwhile. */
  retry(): void {
    if (this.book?.failed) this.resolve(this.book);
  }

  private resolve(current: CurrentBook): void {
    current.failed = false;
    const resolution = this.engine.identityFor(current.local).then(
      (identity) => {
        if (this.book !== current) return;
        this.engine.resolved(current.local, null);
        current.identity = identity ?? null;
        if (identity) this.engine.openBook.set({ title: current.local.title, ...identity });
        const queued = this.unkeyed;
        this.unkeyed = [];
        queued.forEach(({ spans, position, atMs }) => this.emit(spans, position, atMs));
      },
      (error: unknown) => {
        if (this.book !== current) return;
        current.failed = true;
        this.engine.resolved(current.local, error);
      }
    );
    this.engine.track(resolution);
  }

  observe(position: number): void {
    if (!Number.isFinite(position) || position < 0 || this.engine.hidden()) return;
    const atMs = this.engine.now();
    const settling = this.engine.settling(atMs);
    // ttu reports 0 while it lays out a new section, so a page turn into the next section is
    // not a move back to the start. Programmatic moves to 0 still re-anchor.
    if (position === 0 && !settling && (this.trail.anchor?.position ?? 0) > 0) return;
    this.lastPosition = position;
    const policy = this.policy();
    const step = settling
      ? relocate(this.trail, position, atMs, policy)
      : observe(this.trail, position, atMs, policy);
    this.trail = step.trail;
    this.emit(step.kind === 'read' ? step.spans : SpanSet.empty, position, atMs);
  }

  private policy() {
    const threshold = this.options.forwardSkipThreshold();
    return {
      forwardSkipThreshold:
        Number.isFinite(threshold) && threshold > 0 ? threshold : DEFAULT_SKIP_THRESHOLD
    };
  }

  release(): void {
    this.trail = release(this.trail);
  }

  async close(): Promise<void> {
    this.engine.unbind(this);
    await this.engine.flush();
  }

  private emit(spans: SpanSet, position: number, atMs: number): void {
    const book = this.book;
    if (!book || book.identity === null) return;
    if (book.identity === undefined) {
      this.unkeyed.push({ spans, position, atMs });
      return;
    }
    const { key, length } = book.identity;
    this.engine.record({
      book: key,
      day: readingDayOf(new Date(atMs)),
      title: book.local.title,
      length,
      spans: spans.below(length),
      position: Math.min(position, length),
      atMs
    });
  }
}
