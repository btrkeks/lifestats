/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */

import type BooksDbV7 from '$lib/data/database/books-db/versions/v7/books-db-v7';
import type {
  BooksDbV7EbookSyncQueueEntry,
  BooksDbV7EbookSyncState
} from '$lib/data/database/books-db/versions/v7/books-db-v7';

export type BooksDb = BooksDbV7;

export type BooksDbBookData = BooksDb['data']['value'];
export type BooksDbBookmarkData = BooksDb['bookmark']['value'];
export type BooksDbStorageSource = BooksDb['storageSource']['value'];
export type BooksDbStatistic = BooksDb['statistic']['value'];
export type BooksDbReadingGoal = BooksDb['readingGoal']['value'];
export type BooksDbLastModified = BooksDb['lastModified']['value'];
export type BooksDbAudioBook = BooksDb['audioBook']['value'];
export type BooksDbSubtitleData = BooksDb['subtitle']['value'];
export type BooksDbHandle = BooksDb['handle']['value'];
export type BooksDbEbookSyncState = BooksDbV7EbookSyncState;
export type BooksDbEbookSyncQueueEntry = BooksDbV7EbookSyncQueueEntry;
export const currentDbVersion = 7;
