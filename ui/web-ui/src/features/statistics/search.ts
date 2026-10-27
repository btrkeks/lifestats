import type { StatisticMetadata } from '@/types';

/** Case- and width-insensitive form for matching (全角 "ＡＢ" matches "ab"). */
function fold(text: string): string {
  return text.normalize('NFKC').toLocaleLowerCase();
}

/** Statistics whose name or category contains the query; all of them for a blank query. */
export function searchStatistics(
  statistics: StatisticMetadata[],
  query: string
): StatisticMetadata[] {
  const needle = fold(query.trim());
  if (!needle) return statistics;
  return statistics.filter(
    (statistic) =>
      fold(statistic.name).includes(needle) || fold(statistic.category).includes(needle)
  );
}

export interface TextPart {
  text: string;
  match: boolean;
}

/** Splits `text` around case-insensitive occurrences of `query`, for <mark> highlights. */
export function highlightParts(text: string, query: string): TextPart[] {
  const needle = fold(query.trim());
  if (!needle) return [{ text, match: false }];
  // Folding can change lengths (NFKC), so match on a folded copy only when it doesn't.
  const haystack = fold(text);
  if (haystack.length !== text.length) return [{ text, match: false }];

  const parts: TextPart[] = [];
  let from = 0;
  for (let at = haystack.indexOf(needle); at !== -1; at = haystack.indexOf(needle, from)) {
    if (at > from) parts.push({ text: text.slice(from, at), match: false });
    parts.push({ text: text.slice(at, at + needle.length), match: true });
    from = at + needle.length;
  }
  if (from < text.length) parts.push({ text: text.slice(from), match: false });
  return parts;
}
