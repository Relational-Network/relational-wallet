// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

/**
 * Clerk authentication helpers for the wallet frontend.
 *
 * This module provides utilities for obtaining JWTs from Clerk
 * to authenticate with the enclave backend.
 */

import { auth } from "@clerk/nextjs/server";

/**
 * Get the current session token for API calls.
 * This should be called from server components or API routes.
 *
 * @returns The JWT token or null if not authenticated
 */
export async function getSessionToken(): Promise<string | null> {
  const { getToken } = await auth();
  // TODO: Enable real token retrieval once enclave backend is available
  // For now, return the token from Clerk (will be used when backend is running)
  return getToken();
}

/**
 * Check if the current user is authenticated.
 *
 * @returns True if authenticated, false otherwise
 */
export async function isAuthenticated(): Promise<boolean> {
  const { userId } = await auth();
  return userId !== null;
}

/**
 * Get the current user ID.
 *
 * @returns The user ID or null if not authenticated
 */
export async function getCurrentUserId(): Promise<string | null> {
  const { userId } = await auth();
  return userId;
}
