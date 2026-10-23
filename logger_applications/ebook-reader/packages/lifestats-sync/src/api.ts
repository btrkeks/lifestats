/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { DeliveryProblem } from './domain/problem';
import type { ServerUrlParse } from './domain/server-url';

export type { DeliveryProblem };

/** Svelte's store contract, without depending on Svelte. */
export interface Readable<T> {
  subscribe(run: (value: T) => void): () => void;
}

/** A book in ttu's library, as the sync needs it. */
export interface LocalBook {
  readonly localId: number;
  readonly title: string;
  /** ttu's `lastBookModified`. A re-import changes it, so the key is recomputed. */
  readonly modified: number;
  /** The text ttu's character counter walks. Called at most once per book version. */
  countedText(): string;
}

export interface Library {
  book(localId: number): Promise<LocalBook | undefined>;
}

/** A delta the retired v7 patch queued for the legacy events route. */
export interface LegacyQueued {
  readonly eventId: string;
  readonly readerDataId: number;
  readonly day: string;
  readonly delta: number;
  readonly positionAfter: number;
  readonly targetCharacters: number | null;
  readonly title: string;
  readonly createdAt: number;
}

/** A v7 tracker position. Reading past `flushedPosition` was confirmed but never queued. */
export interface LegacyTail {
  readonly readerDataId: number;
  readonly flushedPosition: number;
  readonly confirmedPosition: number;
  readonly updatedAt: number;
}

/** What the retired v7 patch left in this origin. Only ever read. */
export interface LegacySource {
  queued(): Promise<readonly LegacyQueued[]>;
  tails(): Promise<readonly LegacyTail[]>;
  /** ttu's stored title and character count, while the book is in the library. */
  book(
    readerDataId: number
  ): Promise<{ readonly title: string; readonly characters: number } | undefined>;
  /** The v7 day rule: ttu's `getDateKey` with its start-of-day hour. */
  dayOf(atMs: number): string;
}

export interface SyncInit {
  /** SvelteKit's build version. A new build re-arms parked records. */
  readonly buildId: string;
  readonly library: Library;
  readonly legacy: LegacySource | null;
}

export interface ReaderOptions {
  /** ttu's `trackerForwardSkipThreshold`, read on every step so changes apply at once. */
  readonly forwardSkipThreshold: () => number;
}

/** One per reader page. All calls are cheap and synchronous; storage happens behind them. */
export interface ReaderBinding {
  setBook(book: LocalBook | null): void;
  /** Every position change. The first one after `relocate()` only moves the anchor. */
  observe(position: number): void;
  close(): Promise<void>;
}

export interface OpenBook {
  readonly title: string;
  readonly key: string;
  readonly length: number;
}

export interface PendingSummary {
  /** (book, day) records the current server has not acknowledged. */
  readonly records: number;
  /** Characters in those records, before the server removes rereading on other devices. */
  readonly characters: number;
  readonly oldestDay: string | null;
  /** Reading saved by an earlier version of this reader, not yet delivered. */
  readonly older: number;
}

/** A book whose reading stays on this device until someone fixes `problem`. */
export interface BlockedBook {
  readonly title: string;
  readonly problem: DeliveryProblem;
}

interface Known {
  readonly pending: PendingSummary;
  readonly blockedBooks: readonly BlockedBook[];
  readonly lastDeliveredAt: number | null;
}

/** What the indicator and panel render. Derived from storage on every change. */
export type SyncStatus =
  | { readonly kind: 'starting' }
  | (Known & { readonly kind: 'storage_failed'; readonly message: string })
  | (Known & { readonly kind: 'not_configured' })
  | (Known & { readonly kind: 'synced' })
  | (Known & { readonly kind: 'sending' })
  | (Known & {
      readonly kind: 'waiting';
      readonly problem: DeliveryProblem;
      readonly nextAttemptAt: number;
    })
  | (Known & { readonly kind: 'blocked'; readonly problem: DeliveryProblem });

export type ConfigureResult =
  | { readonly ok: true; readonly serverUrl: string | null }
  | Extract<ServerUrlParse, { ok: false }>;

export type ConnectionCheck =
  | { readonly kind: 'ok'; readonly serverId: string }
  | Extract<ServerUrlParse, { ok: false }>
  | { readonly kind: 'problem'; readonly problem: DeliveryProblem };

export interface LifestatsSync {
  reader(options: ReaderOptions): ReaderBinding;
  /** Programmatic movement starts now: open, restore, resize, navigation. Never credits. */
  relocate(): void;
  readonly status: Readable<SyncStatus>;
  readonly serverUrl: Readable<string | null>;
  readonly openBook: Readable<OpenBook | null>;
  configure(serverUrl: string | null): Promise<ConfigureResult>;
  testConnection(serverUrl: string): Promise<ConnectionCheck>;
  retryNow(): Promise<void>;
  /** Commits, delivers once, and settles within `timeoutMs`. Never rejects. */
  drain(timeoutMs: number): Promise<'idle' | 'pending' | 'timeout'>;
}
