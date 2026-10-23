/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

describe('@lifestats/ttu-sync boundary', () => {
  it('exposes its public entry', async () => {
    const entry = await import('@lifestats/ttu-sync');
    expect(Object.keys(entry).sort()).toEqual([
      'isLoopbackServerUrl',
      'lifestatsSync',
      'startLifestatsSync'
    ]);
  });

  it.each(['src/store/sync-db', 'src/wire/http', 'src/engine/engine'])(
    'keeps %s unreachable from the app',
    async (internal) => {
      const specifier = `@lifestats/ttu-sync/${internal}`;
      await expect(import(/* @vite-ignore */ specifier)).rejects.toThrow();
    }
  );

  it('names no storage type on its public surface', () => {
    for (const file of ['api.ts', 'index.ts']) {
      const source = readFileSync(
        new URL(`../../../../../packages/lifestats-sync/src/${file}`, import.meta.url),
        'utf8'
      );
      expect(source).not.toMatch(/\bIDB\w*|from 'idb'/);
    }
  });
});
