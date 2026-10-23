/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { version } from '$app/environment';
import type { BooksDbBookData } from '$lib/data/database/books-db/versions/books-db';
import { database, startDayHoursForTracker$ } from '$lib/data/store';
import { getDateKey } from '$lib/functions/statistic-util';
import {
  startLifestatsSync,
  type LifestatsSync,
  type LocalBook,
  type ReaderBinding,
  type ReaderOptions
} from '@lifestats/ttu-sync';
import { countedBookText } from './counted-text';
import { legacySource } from './legacy-source';
import { withoutSectionLayout } from './section-layout';

let sync: LifestatsSync | undefined;
let reader: ReturnType<typeof withoutSectionLayout> | undefined;

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

/** The reader page's binding. One per page. */
export function startReader(options: ReaderOptions): ReaderBinding {
  reader = withoutSectionLayout(startSync().reader(options));
  return reader.binding;
}

/** The paginated reader is laying out a section; its position until then is not reading. */
export function sectionLayout(started: boolean): void {
  reader?.layout(started);
}

/** A page turn found no next section: the last screen was read. */
export function readingReachedEnd(): void {
  reader?.binding.reachedEnd();
}
