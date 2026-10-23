/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

export type ServerUrl = string & { readonly __brand: 'ServerUrl' };

export type ServerUrlParse =
  | { readonly ok: true; readonly url: ServerUrl }
  | {
      readonly ok: false;
      readonly reason: 'not_http_url' | 'has_credentials' | 'has_query' | 'has_fragment';
    };

/** http(s) origin with an optional path prefix, without a trailing slash. */
export function parseServerUrl(raw: string): ServerUrlParse {
  let url: URL;
  try {
    url = new URL(raw.trim());
  } catch {
    return { ok: false, reason: 'not_http_url' };
  }
  if (url.protocol !== 'http:' && url.protocol !== 'https:') {
    return { ok: false, reason: 'not_http_url' };
  }
  if (url.username || url.password) return { ok: false, reason: 'has_credentials' };
  if (url.search || raw.includes('?')) return { ok: false, reason: 'has_query' };
  if (url.hash || raw.includes('#')) return { ok: false, reason: 'has_fragment' };
  const path = url.pathname.replace(/\/+$/, '');
  return { ok: true, url: `${url.origin}${path}` as ServerUrl };
}

/** Whether the URL names this device, which on a phone is not the laptop running Lifestats. */
export function isLoopback(url: ServerUrl): boolean {
  const { hostname } = new URL(url);
  return hostname === 'localhost' || hostname === '[::1]' || hostname.startsWith('127.');
}
