// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

/**
 * Email hashing utilities for the client side.
 *
 * The server never sees the raw email — the client normalizes the email,
 * computes a SHA-256 hash, and sends only the hash to the server.
 * The server then applies HMAC to produce the actual lookup key.
 */

/**
 * Normalize an email address for consistent hashing:
 * - Trim whitespace
 * - Lowercase
 * - NFC unicode normalization (via String.normalize)
 */
export function normalizeEmail(email: string): string {
  return email.trim().toLowerCase().normalize('NFC');
}

/**
 * Compute SHA-256 hash of a normalized email address.
 * Returns a 64-character lowercase hex string.
 */
export async function hashEmail(email: string): Promise<string> {
  const normalized = normalizeEmail(email);
  const encoder = new TextEncoder();
  const data = encoder.encode(normalized);
  const hashBuffer = await crypto.subtle.digest('SHA-256', data);
  const hashArray = Array.from(new Uint8Array(hashBuffer));
  return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Mask an email address for display purposes.
 * e.g. "alice@example.com" → "a***e@example.com"
 */
export function maskEmail(email: string): string {
  const normalized = normalizeEmail(email);
  const atIndex = normalized.indexOf('@');
  if (atIndex < 0) return '***';

  const local = normalized.slice(0, atIndex);
  const domain = normalized.slice(atIndex);

  if (local.length <= 2) {
    return `${local[0]}***${domain}`;
  }
  return `${local[0]}***${local[local.length - 1]}${domain}`;
}
