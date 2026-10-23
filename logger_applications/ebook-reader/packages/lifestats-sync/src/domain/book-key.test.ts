/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { createHash } from 'node:crypto';
import { describe, expect, it } from 'vitest';

import { bookIdentity } from './book-key';

describe('bookIdentity', () => {
  it('hashes the UTF-8 counted text into the server key format', async () => {
    const text = '吾輩は猫である名前はまだ無い';
    const expected = createHash('sha256').update(text, 'utf8').digest('hex');
    expect(await bookIdentity(text)).toEqual({ key: `ttu-text-v1:${expected}`, length: 14 });
  });

  it('counts code points, like ttu', async () => {
    expect((await bookIdentity('𠮟る'))?.length).toBe(2);
  });

  it('gives the same key for the same text and a different key for different text', async () => {
    const a = await bookIdentity('猫である');
    expect(await bookIdentity('猫である')).toEqual(a);
    expect((await bookIdentity('犬である'))?.key).not.toBe(a?.key);
  });

  it('has no identity for a book with nothing countable', async () => {
    expect(await bookIdentity('')).toBeUndefined();
  });
});
