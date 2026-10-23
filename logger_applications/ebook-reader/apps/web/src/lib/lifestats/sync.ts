/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { version } from '$app/environment';
import type { BooksDbBookData } from '$lib/data/database/books-db/versions/books-db';
import { database, startDayHoursForTracker$ } from '$lib/data/store';
import { getDateKey } from '$lib/functions/statistic-util';
import { startLifestatsSync, type LifestatsSync, type LocalBook } from '@lifestats/ttu-sync';
import { countedBookText } from './counted-text';
import { legacySource } from './legacy-source';

let sync: LifestatsSync | undefined;

export function toLocalBook(data: BooksDbBookData): LocalBook {
  return {
    localId: data.id,
    title: data.title,
    modified: data.lastBookModified,
    countedText: () => countedBookText(data.elementHtml, document)
  };
}

/** Runs once per page, from the root layout, so every ttu page delivers. */
export function startSync(): LifestatsSync {
  sync ??= startLifestatsSync({
    buildId: version,
    library: {
      async book(localId) {
        const data = await database.getData(localId);
        return data && toLocalBook(data);
      }
    },
    legacy: legacySource(database.db, (atMs) =>
      getDateKey(startDayHoursForTracker$.getValue(), new Date(atMs))
    )
  });
  return sync;
}

/** Programmatic movement in a reader: opening, restoring, resizing, navigating. Never credits. */
export function relocateReading(): void {
  sync?.relocate();
}
