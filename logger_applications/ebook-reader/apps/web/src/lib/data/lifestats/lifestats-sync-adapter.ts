/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */

import type {
  BooksDbEbookSyncQueueEntry,
  BooksDbEbookSyncState
} from '$lib/data/database/books-db/versions/books-db';
import type { ReadingProgressTrackerState } from '$lib/data/lifestats/reading-progress-tracker';
import { DatabaseService } from '$lib/data/database/books-db/database.service';

const EBOOK_API_BASE_URL = 'http://localhost:3000';
const EBOOK_BUNDLE_VERSION = 1;

export class LifestatsSyncAdapter {
  private bundleEnsured = false;

  constructor(
    private readonly database: DatabaseService,
    private readonly apiBaseUrl = EBOOK_API_BASE_URL
  ) {}

  buildSyncState(dataId: number, trackerState: ReadingProgressTrackerState): BooksDbEbookSyncState {
    return {
      dataId,
      currentPosition: trackerState.currentPosition,
      confirmedPosition: trackerState.confirmedPosition,
      lastFlushedPosition: trackerState.lastFlushedPosition,
      pendingJump: trackerState.pendingJump,
      bundleVersion: EBOOK_BUNDLE_VERSION,
      diagnosticReason: trackerState.diagnosticReason,
      updatedAt: Date.now()
    };
  }

  createQueueEntry(
    dataId: number,
    dateKey: string,
    delta: number,
    confirmedPositionAfter: number,
    bookTitle: string
  ): BooksDbEbookSyncQueueEntry {
    return {
      eventId: crypto.randomUUID(),
      dataId,
      dateKey,
      metric: 'characters_read',
      delta,
      confirmedPositionAfter,
      bookTitle,
      createdAt: Date.now(),
      attemptCount: 0,
      nextRetryAt: 0
    };
  }

  async persistState(syncState: BooksDbEbookSyncState) {
    await this.database.putEbookSyncState(syncState);
  }

  async persistConfirmedDelta(
    syncState: BooksDbEbookSyncState,
    queueEntry: BooksDbEbookSyncQueueEntry
  ) {
    await this.database.commitEbookSyncDelta(syncState, queueEntry);
  }

  async ensureBundle() {
    if (this.bundleEnsured) {
      return;
    }

    const response = await fetch(`${this.apiBaseUrl}/integrations/ebook`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: '{}'
    });

    if (!response.ok) {
      throw await this.buildResponseError(response, 'Failed to provision ebook bundle');
    }

    this.bundleEnsured = true;
  }

  async flushQueue(force = false) {
    const queueEntries = await this.database.getEbookSyncQueueEntries(force, Date.now());
    let flushed = 0;

    for (let index = 0; index < queueEntries.length; index += 1) {
      const queueEntry = queueEntries[index];

      try {
        await this.ensureBundle();

        const response = await fetch(`${this.apiBaseUrl}/integrations/ebook/events`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json'
          },
          body: JSON.stringify({
            event_id: queueEntry.eventId,
            date: queueEntry.dateKey,
            metric: queueEntry.metric,
            delta: queueEntry.delta,
            book_id: queueEntry.dataId,
            book_title: queueEntry.bookTitle,
            confirmed_position_after: queueEntry.confirmedPositionAfter
          })
        });

        if (!response.ok) {
          throw await this.buildResponseError(response, 'Failed to flush ebook sync event');
        }

        await this.database.deleteEbookSyncQueueEntry(queueEntry.eventId);
        flushed += 1;
      } catch (error: any) {
        await this.database.putEbookSyncQueueEntry({
          ...queueEntry,
          attemptCount: queueEntry.attemptCount + 1,
          nextRetryAt: Date.now() + this.retryDelayMs(queueEntry.attemptCount + 1),
          lastError: error?.message || 'Unknown ebook sync error'
        });
      }
    }

    return {
      flushed,
      queued: queueEntries.length
    };
  }

  private retryDelayMs(attemptCount: number) {
    if (attemptCount <= 1) {
      return 5_000;
    }

    if (attemptCount === 2) {
      return 15_000;
    }

    if (attemptCount === 3) {
      return 60_000;
    }

    return 300_000;
  }

  private async buildResponseError(response: Response, prefix: string) {
    let responseBody = '';

    try {
      responseBody = (await response.text()).trim();
    } catch {
      responseBody = '';
    }

    const details = responseBody ? `: ${responseBody}` : '';
    return new Error(`${prefix} (${response.status} ${response.statusText})${details}`);
  }
}
