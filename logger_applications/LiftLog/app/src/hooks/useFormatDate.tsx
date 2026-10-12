import { LocalDate } from '@js-joda/core';
import { useAppSelector } from '@/store';
import { useCallback } from 'react';
import { formatDate } from '@/utils/format-date';

export function useFormatDate(): (date: LocalDate, opts: Intl.DateTimeFormatOptions) => string {
  const locale = useAppSelector((x) => x.settings.preferredLanguage);
  return useCallback((date, opts) => formatDate(locale, date, opts), [locale]);
}
