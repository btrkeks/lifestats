/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */

import type BooksDbV6 from '$lib/data/database/books-db/versions/v6/books-db-v6';

export interface EbookSyncPendingJump {
  fromPosition: number;
  targetPosition: number;
  furthestPosition: number;
  dwellSeconds: number;
  observedAt: number;
}

export interface BooksDbV7EbookSyncState {
  dataId: number;
  currentPosition: number;
  confirmedPosition: number;
  lastFlushedPosition: number;
  pendingJump?: EbookSyncPendingJump;
  bundleVersion: number;
  diagnosticReason?: string;
  updatedAt: number;
}

export interface BooksDbV7EbookSyncQueueEntry {
  eventId: string;
  dataId: number;
  dateKey: string;
  metric: 'characters_read';
  delta: number;
  confirmedPositionAfter: number;
  bookTitle: string;
  createdAt: number;
  attemptCount: number;
  nextRetryAt: number;
  lastError?: string;
}

export default interface BooksDbV7 extends BooksDbV6 {
  ebookSyncState: {
    key: number;
    value: BooksDbV7EbookSyncState;
  };
  ebookSyncQueue: {
    key: string;
    value: BooksDbV7EbookSyncQueueEntry;
    indexes: {
      nextRetryAt: number;
      dataId: number;
    };
  };
}
