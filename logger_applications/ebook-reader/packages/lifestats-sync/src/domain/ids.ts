/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

declare const brand: unique symbol;
type Brand<T, B extends string> = T & { readonly [brand]: B };

/** One per ttu origin database. A producer only ever writes its own server rows. */
export type ProducerId = Brand<string, 'ProducerId'>;

/** The database identity a Lifestats server reports from `GET /integrations/ebook`. */
export type ServerId = Brand<string, 'ServerId'>;

/** `ttu-text-v1:` plus the SHA-256 of the book's counted text. */
export type BookKey = Brand<string, 'BookKey'>;

/** `YYYY-MM-DD` of the local day, which starts at 04:00. */
export type ReadingDay = Brand<string, 'ReadingDay'>;

/** Same rollover as Yomikata, so both integrations share day boundaries on the dashboard. */
export const READING_DAY_ROLLOVER_HOUR = 4;

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const bookKey = /^ttu-text-v1:[0-9a-f]{64}$/;
const day = /^(\d{4})-(\d{2})-(\d{2})$/;

function isUuid(raw: unknown): raw is string {
  return typeof raw === 'string' && uuid.test(raw);
}

export function isProducerId(raw: unknown): raw is ProducerId {
  return isUuid(raw);
}

export function newProducerId(): ProducerId {
  const id = crypto.randomUUID().toLowerCase();
  if (!isProducerId(id)) throw new Error('crypto.randomUUID returned a non-canonical UUID');
  return id;
}

export function isServerId(raw: unknown): raw is ServerId {
  return isUuid(raw);
}

export function isBookKey(raw: unknown): raw is BookKey {
  return typeof raw === 'string' && bookKey.test(raw);
}

export function isReadingDay(raw: unknown): raw is ReadingDay {
  const match = typeof raw === 'string' ? day.exec(raw) : null;
  if (!match) return false;
  const [, year, month, date] = match.map(Number);
  const parsed = new Date(Date.UTC(year, month - 1, date));
  return (
    parsed.getUTCFullYear() === year &&
    parsed.getUTCMonth() === month - 1 &&
    parsed.getUTCDate() === date
  );
}

/**
 * Local calendar date, moved back one day before 04:00. Uses calendar fields rather than
 * subtracting four hours, so DST days keep their boundary at 04:00 local time.
 */
export function readingDayOf(at: Date): ReadingDay {
  const local = new Date(at.getFullYear(), at.getMonth(), at.getDate());
  if (at.getHours() < READING_DAY_ROLLOVER_HOUR) local.setDate(local.getDate() - 1);
  const label = [
    String(local.getFullYear()).padStart(4, '0'),
    String(local.getMonth() + 1).padStart(2, '0'),
    String(local.getDate()).padStart(2, '0')
  ].join('-');
  if (!isReadingDay(label)) throw new Error(`Invalid reading day ${label}`);
  return label;
}
