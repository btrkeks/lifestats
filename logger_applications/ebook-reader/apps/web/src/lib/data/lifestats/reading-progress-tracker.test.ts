import { describe, expect, it } from 'vitest';

import { ReadingProgressTracker } from './reading-progress-tracker';

const trackerConfig = {
  suspiciousForwardJumpThreshold: 400,
  jumpDwellSeconds: 15,
  jumpConfirmationForwardChars: 120
};

describe('ReadingProgressTracker', () => {
  it('uses the current position as the baseline when resuming', () => {
    const tracker = new ReadingProgressTracker(
      {
        currentPosition: 0,
        confirmedPosition: 0,
        lastFlushedPosition: 0
      },
      trackerConfig
    );

    tracker.pause();
    tracker.resumeAt(12_000);

    expect(tracker.snapshot()).toMatchObject({
      currentPosition: 12_000,
      confirmedPosition: 12_000,
      lastFlushedPosition: 12_000,
      flushableCharacters: 0,
      diagnosticReason: 'resumed_at_baseline'
    });
  });

  it('only confirms active progress after the resume baseline', () => {
    const tracker = new ReadingProgressTracker(
      {
        currentPosition: 0,
        confirmedPosition: 0,
        lastFlushedPosition: 0
      },
      trackerConfig
    );

    tracker.pause();
    tracker.resumeAt(12_000);
    const snapshot = tracker.tick(1, 12_080);

    expect(snapshot).toMatchObject({
      currentPosition: 12_080,
      confirmedPosition: 12_080,
      lastFlushedPosition: 12_000,
      flushableCharacters: 80,
      diagnosticReason: 'forward_progress_confirmed'
    });
  });

  it('clears pending jumps when resuming at a baseline', () => {
    const tracker = new ReadingProgressTracker(
      {
        currentPosition: 0,
        confirmedPosition: 0,
        lastFlushedPosition: 0
      },
      trackerConfig
    );

    tracker.tick(1, 12_000);
    expect(tracker.snapshot().pendingJump).toBeDefined();

    tracker.pause();
    tracker.resumeAt(12_000);

    expect(tracker.snapshot()).toMatchObject({
      pendingJump: undefined,
      flushableCharacters: 0,
      diagnosticReason: 'resumed_at_baseline'
    });
  });

  it('does not create negative flushable characters when resuming earlier', () => {
    const tracker = new ReadingProgressTracker(
      {
        currentPosition: 1_500,
        confirmedPosition: 1_500,
        lastFlushedPosition: 1_500
      },
      trackerConfig
    );

    tracker.pause();
    tracker.resumeAt(800);

    expect(tracker.snapshot()).toMatchObject({
      currentPosition: 800,
      confirmedPosition: 800,
      lastFlushedPosition: 800,
      flushableCharacters: 0
    });
  });
});
