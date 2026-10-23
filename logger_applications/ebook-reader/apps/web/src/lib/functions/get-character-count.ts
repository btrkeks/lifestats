/**
 * @license BSD-3-Clause
 * Copyright (c) 2024, ッツ Reader Authors
 * All rights reserved.
 */

import { isNodeGaiji } from './is-node-gaiji';

/** Stands for one gaiji image in counted text. The filter below removes it from real text. */
export const gaijiPlaceholder = '\uFFFC';

export function getCharacterCount(node: Node) {
  return countUnicodeCharacters(getCountedText(node));
}

const isNotJapaneseRegex =
  /[^0-9A-Z○◯々-〇〻ぁ-ゖゝ-ゞァ-ヺー０-９Ａ-Ｚｦ-ﾝ\p{Radical}\p{Unified_Ideograph}]+/gimu;

/** The characters of `node` that the counter counts, in order. */
export function getCountedText(node: Node) {
  if (isNodeGaiji(node)) return gaijiPlaceholder;
  if (!node.textContent) return '';
  return node.textContent.replace(isNotJapaneseRegex, '');
}

/**
 * Because '𠮟る'.length = 3
 * Reference: https://dmitripavlutin.com/what-every-javascript-developer-should-know-about-unicode/#length-and-surrogate-pairs
 */
function countUnicodeCharacters(s: string) {
  return Array.from(s).length;
}
