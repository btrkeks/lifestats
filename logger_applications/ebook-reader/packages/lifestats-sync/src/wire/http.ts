/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { DayRecord } from '../domain/day-record';
import type { AttemptOutcome, FailedAttempt } from '../domain/delivery-policy';
import { isServerId, type BookKey, type ProducerId, type ServerId } from '../domain/ids';
import type { ServerUrl } from '../domain/server-url';
import type { components } from './schema';

type Schemas = components['schemas'];
type DayRequest = Schemas['EbookDayRequest'];
export const REQUEST_TIMEOUT_MS = 15_000;

export type Description = { readonly kind: 'ok'; readonly serverId: ServerId } | FailedAttempt;

type Sent = { readonly kind: 'response'; readonly response: Response } | FailedAttempt;

async function send(
  fetchImpl: typeof fetch,
  base: ServerUrl,
  path: string,
  init: { method: 'GET' | 'PUT'; body?: unknown }
): Promise<Sent> {
  try {
    const response = await fetchImpl(`${base}${path}`, {
      method: init.method,
      headers: init.body === undefined ? undefined : { 'Content-Type': 'application/json' },
      body: init.body === undefined ? undefined : JSON.stringify(init.body),
      redirect: 'manual',
      cache: 'no-store',
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS)
    });
    return { kind: 'response', response };
  } catch {
    return {
      kind: 'retryable',
      problem: { kind: 'unreachable', serverUrl: base },
      retryAfterMs: null
    };
  }
}

function retryAfterMs(response: Response): number | null {
  const seconds = Number(response.headers.get('Retry-After'));
  return Number.isFinite(seconds) && seconds > 0 ? seconds * 1000 : null;
}

async function rejection(response: Response): Promise<FailedAttempt> {
  const body: unknown = await response.json().catch(() => undefined);
  const error =
    typeof body === 'object' && body !== null && 'error' in body ? body.error : undefined;
  const field = (key: string) =>
    typeof error === 'object' && error !== null && key in error
      ? (error as Record<string, unknown>)[key]
      : undefined;
  const code = field('code');
  const message = field('message');
  const name = field('field');
  return {
    kind: 'needs_fix',
    problem: {
      kind: 'rejected',
      httpStatus: response.status,
      code: typeof code === 'string' ? code : `http_${response.status}`,
      field: typeof name === 'string' ? name : null,
      message: typeof message === 'string' ? message : response.statusText
    }
  };
}

async function classify(sent: Sent, base: ServerUrl): Promise<AttemptOutcome> {
  if (sent.kind !== 'response') return sent;
  return sent.response.ok ? { kind: 'delivered' } : failure(sent.response, base);
}

/** 408, 429 and 5xx retry; a missing route means an older server; other answers park. */
async function failure(response: Response, base: ServerUrl): Promise<FailedAttempt> {
  if (response.type === 'opaqueredirect' || (response.status >= 300 && response.status < 400)) {
    return {
      kind: 'needs_fix',
      problem: {
        kind: 'rejected',
        httpStatus: response.status,
        code: 'redirect',
        field: null,
        message: 'The server answered with a redirect'
      }
    };
  }
  if (response.status === 408 || response.status === 429 || response.status >= 500) {
    return {
      kind: 'retryable',
      problem: { kind: 'server_error', httpStatus: response.status },
      retryAfterMs: retryAfterMs(response)
    };
  }
  if (response.status === 404 || response.status === 405) {
    return { kind: 'needs_fix', problem: { kind: 'server_too_old', serverUrl: base } };
  }
  return rejection(response);
}

export async function describeServer(
  fetchImpl: typeof fetch,
  base: ServerUrl
): Promise<Description> {
  const sent = await send(fetchImpl, base, '/integrations/ebook', { method: 'GET' });
  if (sent.kind !== 'response') return sent;
  const { response } = sent;
  if (response.status === 404) {
    return { kind: 'needs_fix', problem: { kind: 'not_lifestats', serverUrl: base } };
  }
  if (!response.ok) return failure(response, base);
  const body: unknown = await response.json().catch(() => undefined);
  const descriptor =
    typeof body === 'object' && body !== null && 'server_id' in body && 'protocol' in body
      ? body
      : undefined;
  if (!descriptor || !isServerId(descriptor.server_id)) {
    return { kind: 'needs_fix', problem: { kind: 'not_lifestats', serverUrl: base } };
  }
  if (descriptor.protocol !== 1) {
    return {
      kind: 'needs_fix',
      problem: {
        kind: 'rejected',
        httpStatus: response.status,
        code: 'unsupported_protocol',
        field: 'protocol',
        message: `Server speaks ebook protocol ${String(descriptor.protocol)}; this reader speaks 1`
      }
    };
  }
  return { kind: 'ok', serverId: descriptor.server_id };
}

export function dayPath(producer: ProducerId, book: BookKey, day: string): string {
  return `/integrations/ebook/producers/${producer}/books/${encodeURIComponent(book)}/days/${day}`;
}

export async function putDay(
  fetchImpl: typeof fetch,
  base: ServerUrl,
  producer: ProducerId,
  record: DayRecord
): Promise<AttemptOutcome> {
  const body: DayRequest = {
    title: record.title,
    length: record.length,
    spans: record.spans.toPairs(),
    last_position: record.lastPosition,
    observed_at: new Date(record.observedAt).toISOString()
  };
  const path = dayPath(producer, record.book, record.day);
  return classify(await send(fetchImpl, base, path, { method: 'PUT', body }), base);
}
