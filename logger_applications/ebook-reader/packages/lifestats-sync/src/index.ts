/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { LifestatsSync, SyncInit } from './api';
import { isLoopback, parseServerUrl } from './domain/server-url';
import { Engine, type EngineEnvironment } from './engine/engine';
import { SYNC_DB_NAME } from './store/sync-db';

export type {
  BlockedBook,
  ConfigureResult,
  ConnectionCheck,
  DeliveryProblem,
  Library,
  LegacyQueued,
  LegacySource,
  LegacyTail,
  LifestatsSync,
  LocalBook,
  OpenBook,
  PendingSummary,
  Readable,
  ReaderBinding,
  ReaderOptions,
  SyncInit,
  SyncStatus
} from './api';

function browserEnvironment(): EngineEnvironment {
  return {
    fetch: globalThis.fetch.bind(globalThis),
    now: () => Date.now(),
    random: Math.random,
    hidden: () => document.visibilityState === 'hidden',
    listen(listener) {
      const onVisibility = () =>
        listener(document.visibilityState === 'hidden' ? 'hidden' : 'visible');
      const onPageHide = () => listener('hidden');
      const onOnline = () => listener('online');
      const onInput = () => listener('input');
      // Capture, so the input is seen before a ttu handler it triggers calls relocate().
      const input = { capture: true, passive: true };
      const inputs = ['keydown', 'pointerdown', 'wheel', 'touchstart'] as const;
      document.addEventListener('visibilitychange', onVisibility);
      window.addEventListener('pagehide', onPageHide);
      window.addEventListener('online', onOnline);
      inputs.forEach((type) => window.addEventListener(type, onInput, input));
      return () => {
        document.removeEventListener('visibilitychange', onVisibility);
        window.removeEventListener('pagehide', onPageHide);
        window.removeEventListener('online', onOnline);
        inputs.forEach((type) => window.removeEventListener(type, onInput, input));
      };
    },
    locks: navigator.locks ?? null,
    idle: (task) =>
      typeof requestIdleCallback === 'function'
        ? requestIdleCallback(task, { timeout: 2_000 })
        : setTimeout(task, 0),
    timers: {
      set: (task, delayMs) => setTimeout(task, delayMs),
      clear: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>)
    },
    dbName: SYNC_DB_NAME
  };
}

let engine: Engine | undefined;

/**
 * Starts sync for this page once, from the root layout, and installs the host hook
 * `globalThis.ttuLifestats.drain(timeoutMs)` that jidoujisho calls before disposing a page.
 */
export function startLifestatsSync(init: SyncInit): LifestatsSync {
  const started = engine ?? new Engine(init, browserEnvironment());
  engine = started;
  Object.assign(globalThis, { ttuLifestats: { drain: (ms: number) => started.drain(ms) } });
  return started;
}

/** The running sync. Only valid after `startLifestatsSync`. */
export function lifestatsSync(): LifestatsSync {
  if (!engine) throw new Error('startLifestatsSync has not run on this page');
  return engine;
}

/** Whether a server address points at this device, which on a phone is not the laptop. */
export function isLoopbackServerUrl(raw: string): boolean {
  const parsed = parseServerUrl(raw);
  return parsed.ok && isLoopback(parsed.url);
}
