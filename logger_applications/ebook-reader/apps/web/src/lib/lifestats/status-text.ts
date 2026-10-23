/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { DeliveryProblem, SyncStatus } from '@lifestats/ttu-sync';

/** Only jidoujisho's embedded profile turns the service worker off. */
export const embedded = import.meta.env.VITE_SERVICE_WORKER === 'off';

export type Tone = 'quiet' | 'busy' | 'warn' | 'error';

export function problemText(problem: DeliveryProblem, onPhone = embedded): string {
  switch (problem.kind) {
    case 'unreachable':
      return `Lifestats is unreachable at ${problem.serverUrl}.${onPhone ? ' Is Tailscale on?' : ''}`;
    case 'server_error':
      return `Lifestats answered HTTP ${problem.httpStatus}.`;
    case 'not_lifestats':
      return `${problem.serverUrl} is not a Lifestats server.`;
    case 'server_too_old':
      return `The Lifestats server at ${problem.serverUrl} has no reading API yet. Update Lifestats.`;
    case 'rejected':
      return `Lifestats rejected this reading: ${problem.message} (${problem.code}). It stays on this device.`;
    default: {
      const _exhaustive: never = problem;
      return _exhaustive;
    }
  }
}

function plural(count: number, one: string): string {
  return `${count} ${one}${count === 1 ? '' : 's'}`;
}

export function pendingText(status: SyncStatus): string | null {
  if (status.kind === 'starting') return null;
  const { records, characters, legacy } = status.pending;
  const parts = [];
  if (records) parts.push(`${plural(records, 'day')} · ${characters.toLocaleString()} characters`);
  if (legacy) parts.push(plural(legacy, 'older item'));
  return parts.length ? `${parts.join(', ')} waiting` : null;
}

export function statusText(status: SyncStatus, now: number): { tone: Tone; text: string } {
  switch (status.kind) {
    case 'starting':
      return { tone: 'quiet', text: 'Lifestats: starting' };
    case 'storage_failed':
      return {
        tone: 'error',
        text: `Reading is not being saved on this device: ${status.message}`
      };
    case 'not_configured':
      return {
        tone: 'warn',
        text: 'Lifestats: set the server address. Reading is saved on this device until then.'
      };
    case 'synced':
      return { tone: 'quiet', text: 'Lifestats: up to date' };
    case 'sending':
      return { tone: 'busy', text: `Lifestats: sending. ${pendingText(status) ?? ''}`.trim() };
    case 'waiting': {
      const seconds = Math.max(0, Math.ceil((status.nextAttemptAt - now) / 1000));
      return {
        tone: 'warn',
        text: `Lifestats: ${pendingText(status) ?? 'waiting'}. ${problemText(status.problem)} Retrying in ${seconds} s.`
      };
    }
    case 'blocked':
      return {
        tone: 'error',
        text: `Lifestats needs attention. ${problemText(status.problem)}`
      };
    default: {
      const _exhaustive: never = status;
      return _exhaustive;
    }
  }
}
