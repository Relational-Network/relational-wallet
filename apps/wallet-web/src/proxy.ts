// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { clerkMiddleware, createRouteMatcher } from "@clerk/nextjs/server";

/**
 * Route matchers for protected routes.
 * These routes require authentication.
 */
const isProtectedRoute = createRouteMatcher([
  "/wallets(.*)",
  "/account(.*)",
]);

/**
 * Clerk proxy for authentication.
 *
 * Protects:
 * - /wallets and all sub-routes
 * - /account and all sub-routes
 *
 * Public routes:
 * - / (landing page)
 * - /sign-in
 * - /sign-up
 */
export default clerkMiddleware(async (auth, req) => {
  if (isProtectedRoute(req)) {
    await auth.protect();
  }
});

export const config = {
  matcher: [
    // Skip Next.js internals and all static files
    "/((?!_next|[^?]*\\.(?:html?|css|js(?!on)|jpe?g|webp|png|gif|svg|ttf|woff2?|ico|csv|docx?|xlsx?|zip|webmanifest)).*)",
    // Always run for API routes
    "/(api|trpc)(.*)",
  ],
};
