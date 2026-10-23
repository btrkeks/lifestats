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
 */
export function withoutSectionLayout(binding: ReaderBinding) {
  let layingOut = false;
  return {
    binding: {
      setBook: (book) => binding.setBook(book),
      observe(position) {
        if (!layingOut) binding.observe(position);
      },
      close: () => binding.close()
    } satisfies ReaderBinding,
    layout(started: boolean) {
      layingOut = started;
    }
  };
}
