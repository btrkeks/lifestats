/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { SyncStatus } from '../api';
import { isPending, type DayRecord } from './day-record';
import type { DeliveryState } from './delivery-policy';
import type { ServerId } from './ids';
import type { DeliveryProblem } from './problem';

export interface StatusFacts {
  readonly configured: boolean;
  readonly storageError: string | null;
  readonly records: readonly DayRecord[];
  readonly serverId: ServerId | null;
  /** The last descriptor or transport failure, held until it is retried. */
  readonly server: DeliveryState;
  readonly legacy: number;
  readonly missingLegacyBooks: readonly number[];
  readonly claimProblem: DeliveryProblem | null;
  readonly lastDeliveredAt: number | null;
}

export function deriveStatus(facts: StatusFacts): SyncStatus {
  const pendingRecords = facts.records.filter((record) => isPending(record, facts.serverId));
  const known = {
    pending: {
      records: pendingRecords.length,
      characters: pendingRecords.reduce((total, record) => total + record.spans.size, 0),
      oldestDay: pendingRecords.map((record) => record.day).sort()[0] ?? null,
      legacy: facts.legacy,
      missingLegacyBooks: facts.missingLegacyBooks
    },
    lastDeliveredAt: facts.lastDeliveredAt
  };
  if (facts.storageError) return { ...known, kind: 'storage_failed', message: facts.storageError };
  if (!facts.configured) return { ...known, kind: 'not_configured' };
  if (!pendingRecords.length && !facts.legacy) return { ...known, kind: 'synced' };

  const parked = [facts.server, ...pendingRecords.map((record) => record.delivery)].find(
    (state) => state.kind === 'parked'
  );
  if (parked?.kind === 'parked') return { ...known, kind: 'blocked', problem: parked.problem };
  if (facts.claimProblem) return { ...known, kind: 'blocked', problem: facts.claimProblem };

  const retrying = [facts.server, ...pendingRecords.map((record) => record.delivery)]
    .flatMap((state) => (state.kind === 'retrying' ? [state] : []))
    .sort((a, b) => a.nextAttemptAt - b.nextAttemptAt)[0];
  if (retrying) {
    return {
      ...known,
      kind: 'waiting',
      problem: retrying.problem,
      nextAttemptAt: retrying.nextAttemptAt
    };
  }
  return { ...known, kind: 'sending' };
}
