/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { version } from '$app/environment';
import type { BooksDbBookData } from '$lib/data/database/books-db/versions/books-db';
import {
  startLifestatsSync,
  type LifestatsSync,
  type LocalBook,
  type ReaderBinding,
  type ReaderOptions
} from '@lifestats/ttu-sync';
import { countedBookText } from './counted-text';
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
    buildId: version
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

/** The paginated reader is laying out a section; its position until it ends is not reading. */
export function sectionLayoutStarted(): void {
  reader?.startLayout();
}

/** The section is displayed and the reader measured its real position. */
export function sectionLayoutEnded(position: number): void {
  reader?.endLayout(position);
}

/** A page turn found no next section: the last screen was read. */
export function readingReachedEnd(): void {
  reader?.binding.reachedEnd();
}
