/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import { getParagraphNodes } from '$lib/components/book-reader/get-paragraph-nodes';
import { getCountedText } from '$lib/functions/get-character-count';

/**
 * The text ttu's character counter walks for a stored book, in reader order. Its code point
 * count equals the reader's `bookCharCount`, so book keys and positions share one source.
 * Parsed in an inert document so images do not load.
 */
export function countedBookText(elementHtml: string, document: Document): string {
  const container = document.implementation.createHTMLDocument('').createElement('div');
  container.innerHTML = elementHtml;
  return getParagraphNodes(container).map(getCountedText).join('');
}
