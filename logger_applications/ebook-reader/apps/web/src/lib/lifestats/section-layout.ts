/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

import type { ReaderBinding } from '@lifestats/ttu-sync';

/**
 * ttu's paginated reader sets its position to 0 when a section's HTML loads, then measures the
 * real position once the section is displayed. Positions in between are layout, not reading:
 * passing that 0 on would turn a page turn into the next section into a jump back and forth.
 * The measured position is passed on when the layout ends, because the reader reports a
 * position only when it changes, and it may already have reported it during the layout.
 */
export function withoutSectionLayout(binding: ReaderBinding) {
  let layingOut = false;
  return {
    binding: {
      setBook: (book) => binding.setBook(book),
      observe(position) {
        if (!layingOut) binding.observe(position);
      },
      reachedEnd: () => binding.reachedEnd(),
      close: () => binding.close()
    } satisfies ReaderBinding,
    startLayout() {
      layingOut = true;
    },
    endLayout(position: number) {
      layingOut = false;
      binding.observe(position);
    }
  };
}
