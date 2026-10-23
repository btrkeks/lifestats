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

/** The retired v7 patch's leftovers in the `books` database. */
export interface LegacySource {
  /** The app's open `books` connection, at version 8 or later. */
  books(): Promise<IDBDatabase>;
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
  /** Every position change. Within a moment of `relocate()`, it only moves the anchor. */
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
  /** v7 rows not yet drained plus legacy books not yet claimed. */
  readonly legacy: number;
  /** Legacy books gone from this library. Claim them with curl, using a key from another device. */
  readonly missingLegacyBooks: readonly number[];
}

interface Known {
  readonly pending: PendingSummary;
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
