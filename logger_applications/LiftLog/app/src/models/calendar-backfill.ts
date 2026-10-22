import { CalendarTarget, OccurrenceIdentity, Times } from '@/models/calendar-sync';
import { LocalDate } from '@js-joda/core';

export type BackfillRange = { from: LocalDate; to: LocalDate };
export type BackfillRequest = { target: CalendarTarget; range: BackfillRange };
export type BackfillCandidate = { identity: OccurrenceIdentity; current: Times };
export type BackfillWorkout = {
  sessionId: string;
  date: LocalDate;
  name: string;
  completedSpan: Times;
  desired: Times;
};
export type BackfillError = 'no-permission' | 'calendar-missing' | 'unsupported' | 'provider-error';
export type BackfillRow = BackfillWorkout &
  (
    | { status: 'ready'; candidate: BackfillCandidate }
    | { status: 'conflicting-workouts'; candidate: BackfillCandidate }
    | { status: 'unsupported' | 'claimed'; candidate: BackfillCandidate }
    | { status: 'already-linked' | 'missing' | 'ambiguous' }
    | { status: 'error'; reason: BackfillError }
  );
export type BackfillPreview = BackfillRequest & { rows: BackfillRow[] };
export type BackfillApplyRow =
  | { sessionId: string; status: 'queued' | 'already-linked' | 'stale' | 'claimed' | 'not-ready' }
  | { sessionId: string; status: 'error'; reason: BackfillError };
export type BackfillApplyResult = { queued: number; rows: BackfillApplyRow[] };

export function parseBackfillRange(from: string, to: string): BackfillRange | undefined {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(from) || !/^\d{4}-\d{2}-\d{2}$/.test(to)) return undefined;
  try {
    const range = { from: LocalDate.parse(from), to: LocalDate.parse(to) };
    return range.from.isAfter(range.to) ? undefined : range;
  } catch {
    return undefined;
  }
}

export function isBackfillPreviewCurrent(preview: BackfillPreview, request: BackfillRequest | undefined): boolean {
  return (
    request !== undefined &&
    preview.target.calendarId === request.target.calendarId &&
    preview.target.accountName === request.target.accountName &&
    preview.target.title === request.target.title &&
    preview.range.from.equals(request.range.from) &&
    preview.range.to.equals(request.range.to)
  );
}
