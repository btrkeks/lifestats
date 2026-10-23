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
  /** A book the v7 patch counted, gone from this library, so its content key is unknown here. */
  | { readonly kind: 'not_in_library'; readonly readerDataId: number }
  | {
      readonly kind: 'rejected';
      readonly httpStatus: number;
      readonly code: string;
      readonly field: string | null;
      readonly message: string;
    };
