/**
 * @license BSD-3-Clause
 * Copyright (c) 2026, ッツ Reader Authors
 * All rights reserved.
 */

/** Why delivery is not completing. The panel has one sentence per kind. */
export type DeliveryProblem =
  | { readonly kind: 'unreachable'; readonly serverUrl: string }
  | { readonly kind: 'server_error'; readonly httpStatus: number }
  | { readonly kind: 'not_lifestats'; readonly serverUrl: string }
  | { readonly kind: 'server_too_old'; readonly serverUrl: string }
  /** Its content key could not be computed. Its reading waits in memory for Retry now. */
  | { readonly kind: 'untrackable'; readonly message: string }
  | {
      readonly kind: 'rejected';
      readonly httpStatus: number;
      readonly code: string;
      readonly field: string | null;
      readonly message: string;
    };

export function untrackable(error: unknown): DeliveryProblem {
  return { kind: 'untrackable', message: error instanceof Error ? error.message : String(error) };
}
