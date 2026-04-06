/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */

import type { EbookSyncPendingJump } from '$lib/data/database/books-db/versions/v7/books-db-v7';

export interface ReadingProgressTrackerConfig {
  suspiciousForwardJumpThreshold: number;
  jumpDwellSeconds: number;
  jumpConfirmationForwardChars: number;
}

export interface ReadingProgressTrackerState {
  currentPosition: number;
  confirmedPosition: number;
  lastFlushedPosition: number;
  pendingJump?: EbookSyncPendingJump;
  sessionConfirmedCharacters: number;
  flushableCharacters: number;
  diagnosticReason: string;
}

const defaultConfig: ReadingProgressTrackerConfig = {
  suspiciousForwardJumpThreshold: 400,
  jumpDwellSeconds: 15,
  jumpConfirmationForwardChars: 120
};

export class ReadingProgressTracker {
  private currentPosition: number;

  private confirmedPosition: number;

  private lastFlushedPosition: number;

  private pendingJump?: EbookSyncPendingJump;

  private sessionConfirmedCharacters = 0;

  private diagnosticReason = 'initialized';

  private isPaused = false;

  private frozenPosition?: number;

  constructor(
    initialState: Partial<ReadingProgressTrackerState> = {},
    private readonly config: ReadingProgressTrackerConfig = defaultConfig
  ) {
    this.currentPosition = Math.max(0, initialState.currentPosition ?? 0);
    this.confirmedPosition = Math.max(
      0,
      initialState.confirmedPosition ?? initialState.currentPosition ?? 0
    );
    this.lastFlushedPosition = Math.max(
      0,
      Math.min(
        initialState.lastFlushedPosition ?? initialState.confirmedPosition ?? this.confirmedPosition,
        this.confirmedPosition
      )
    );
    this.pendingJump = initialState.pendingJump;
    this.sessionConfirmedCharacters = Math.max(0, initialState.sessionConfirmedCharacters ?? 0);
    this.diagnosticReason = initialState.diagnosticReason ?? this.diagnosticReason;
  }

  pause() {
    this.isPaused = true;
    this.diagnosticReason = 'paused';
  }

  resume() {
    this.isPaused = false;
    this.diagnosticReason = 'resumed';
  }

  freeze(position: number) {
    this.frozenPosition = Math.max(0, position);
    this.currentPosition = this.frozenPosition;
    this.diagnosticReason = 'frozen';
  }

  unfreeze() {
    this.frozenPosition = undefined;
    this.diagnosticReason = 'unfrozen';
  }

  close() {
    this.isPaused = true;
    this.diagnosticReason = 'closed';
  }

  tick(elapsedSeconds: number, exploredCharCount: number, now = Date.now()) {
    if (this.isPaused) {
      return this.snapshot();
    }

    const observedPosition =
      this.frozenPosition === undefined ? exploredCharCount : this.frozenPosition;
    const normalizedObservedPosition = Math.max(0, observedPosition);
    const previousPosition = this.currentPosition;

    this.currentPosition = normalizedObservedPosition;

    if (this.pendingJump) {
      this.updatePendingJump(Math.max(0, elapsedSeconds), now);
      return this.snapshot();
    }

    const positionDiff = normalizedObservedPosition - previousPosition;
    if (positionDiff <= 0) {
      this.diagnosticReason = positionDiff < 0 ? 'backward_motion_ignored' : 'steady';
      return this.snapshot();
    }

    if (positionDiff >= this.config.suspiciousForwardJumpThreshold) {
      this.pendingJump = {
        fromPosition: this.confirmedPosition,
        targetPosition: normalizedObservedPosition,
        furthestPosition: normalizedObservedPosition,
        dwellSeconds: 0,
        observedAt: now
      };
      this.diagnosticReason = 'pending_forward_jump';
      return this.snapshot();
    }

    this.confirmTo(normalizedObservedPosition, 'forward_progress_confirmed');
    return this.snapshot();
  }

  markFlushed(position = this.confirmedPosition) {
    this.lastFlushedPosition = Math.max(0, Math.min(position, this.confirmedPosition));
    this.diagnosticReason = 'flushed';
  }

  snapshot(): ReadingProgressTrackerState {
    return {
      currentPosition: this.currentPosition,
      confirmedPosition: this.confirmedPosition,
      lastFlushedPosition: this.lastFlushedPosition,
      pendingJump: this.pendingJump ? { ...this.pendingJump } : undefined,
      sessionConfirmedCharacters: this.sessionConfirmedCharacters,
      flushableCharacters: Math.max(0, this.confirmedPosition - this.lastFlushedPosition),
      diagnosticReason: this.diagnosticReason
    };
  }

  private updatePendingJump(elapsedSeconds: number, now: number) {
    const pendingJump = this.pendingJump;
    if (!pendingJump) {
      return;
    }

    if (this.currentPosition <= this.confirmedPosition) {
      this.pendingJump = undefined;
      this.diagnosticReason = 'pending_jump_cancelled';
      return;
    }

    if (this.currentPosition < pendingJump.targetPosition) {
      this.pendingJump = {
        ...pendingJump,
        targetPosition: this.currentPosition,
        furthestPosition: Math.max(pendingJump.furthestPosition, this.currentPosition),
        dwellSeconds: 0,
        observedAt: now
      };
      this.diagnosticReason = 'pending_jump_reduced';
      return;
    }

    const updatedPendingJump: EbookSyncPendingJump = {
      ...pendingJump,
      furthestPosition: Math.max(pendingJump.furthestPosition, this.currentPosition),
      dwellSeconds: pendingJump.dwellSeconds + elapsedSeconds,
      observedAt: pendingJump.observedAt || now
    };

    const forwardConfirmationDistance =
      updatedPendingJump.furthestPosition - updatedPendingJump.targetPosition;
    const shouldConfirm =
      updatedPendingJump.dwellSeconds >= this.config.jumpDwellSeconds ||
      forwardConfirmationDistance >= this.config.jumpConfirmationForwardChars;

    if (shouldConfirm) {
      this.pendingJump = undefined;
      this.confirmTo(updatedPendingJump.furthestPosition, 'pending_jump_confirmed');
      return;
    }

    this.pendingJump = updatedPendingJump;
    this.diagnosticReason = 'pending_forward_jump';
  }

  private confirmTo(position: number, diagnosticReason: string) {
    if (position <= this.confirmedPosition) {
      this.diagnosticReason = diagnosticReason;
      return;
    }

    this.sessionConfirmedCharacters += position - this.confirmedPosition;
    this.confirmedPosition = position;
    this.diagnosticReason = diagnosticReason;
  }
}
