import type { IDBPTransaction } from 'idb';

import type { BooksDb } from './versions/books-db';

type EbookSyncCleanupTransaction = IDBPTransaction<
  BooksDb,
  ['ebookSyncState', 'ebookSyncQueue'],
  'readwrite'
>;

export async function deleteEbookSyncRecordsForDataId(
  tx: EbookSyncCleanupTransaction,
  dataId: number,
  deleteQueuedEvents: boolean
) {
  await tx.objectStore('ebookSyncState').delete(dataId);

  if (!deleteQueuedEvents) {
    return;
  }

  const queueStore = tx.objectStore('ebookSyncQueue');
  let cursor = await queueStore.index('dataId').openKeyCursor(IDBKeyRange.only(dataId));

  while (cursor) {
    await queueStore.delete(cursor.primaryKey);
    cursor = await cursor.continue();
  }
}
