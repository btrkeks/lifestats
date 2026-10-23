/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

// @vitest-environment jsdom

import { describe, expect, it } from 'vitest';

import { CharacterStatsCalculator } from '$lib/components/book-reader/book-reader-continuous/character-stats-calculator';
import { countedBookText } from './counted-text';

const html = [
  '<div id="ttu-0"><p>吾輩は<ruby>猫<rt>ねこ</rt></ruby>である。</p>',
  '<p>名前は、まだ無い。 Hello 42!</p></div>',
  '<div id="ttu-1"><p aria-hidden="true">隠れた文</p>',
  '<p>外字<img class="gaiji" src="ttu:g1">と<span hidden>非表示</span>𠮟る</p>',
  '<p><img src="ttu:illustration"></p></div>'
].join('');

function readerCharCount(elementHtml: string) {
  const container = document.createElement('div');
  container.innerHTML = elementHtml;
  document.body.append(container);
  try {
    return new CharacterStatsCalculator(container, 'vertical', 'rtl', container, document)
      .charCount;
  } finally {
    container.remove();
  }
}

describe('countedBookText', () => {
  it('keeps exactly the characters ttu counts, with one placeholder per gaiji', () => {
    expect(countedBookText(html, document)).toBe(
      '吾輩は猫である名前はまだ無いHello42外字\uFFFCと𠮟る'
    );
  });

  it('has as many code points as the reader counts', () => {
    expect(Array.from(countedBookText(html, document)).length).toBe(readerCharCount(html));
  });
});
