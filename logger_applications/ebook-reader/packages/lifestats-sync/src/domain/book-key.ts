/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { isBookKey, type BookKey } from './ids';
import { MAX_OFFSET } from './span-set';

/** A book's identity and the length of the coordinate space its positions live in. */
export interface BookIdentity {
  readonly key: BookKey;
  readonly length: number;
}

/**
 * `countedText` is the text ttu's character counter walks, so `length` equals the reader's
 * `bookCharCount`. Title, file name and the local id play no part. Returns undefined for a
 * book with nothing countable, which the server cannot accept.
 */
export async function bookIdentity(countedText: string): Promise<BookIdentity | undefined> {
  let length = 0;
  for (const _codePoint of countedText) length += 1;
  if (length === 0 || length > MAX_OFFSET) return undefined;
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(countedText));
  const hex = Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, '0'));
  const key = `ttu-text-v1:${hex.join('')}`;
  if (!isBookKey(key)) throw new Error(`Malformed book key ${key}`);
  return { key, length };
}
