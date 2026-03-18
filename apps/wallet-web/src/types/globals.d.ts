// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

/**
 * Clerk custom session claims type augmentation.
 *
 * Requires the following custom session token in Clerk Dashboard
 * (Sessions → Customize session token):
 *
 * ```json
 * {
 *   "publicMetadata": "{{user.public_metadata}}"
 * }
 * ```
 */
export {};

declare global {
  interface CustomJwtSessionClaims {
    publicMetadata?: {
      role?: string;
    };
  }
}
