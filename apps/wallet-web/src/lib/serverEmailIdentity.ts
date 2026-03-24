// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import "server-only";

import { createHash } from "node:crypto";
import { currentUser } from "@clerk/nextjs/server";
import { maskEmail, normalizeEmail } from "@/lib/emailHash";

type CurrentClerkUser = NonNullable<Awaited<ReturnType<typeof currentUser>>>;

export interface EmailLinkIdentity {
  eligible: boolean;
  emailHash: string | null;
  emailDisplay: string | null;
  primaryEmail: string | null;
  warning: string | null;
  emailCount: number;
}

export function getEmailLinkIdentityFromClerkUser(
  clerkUser: CurrentClerkUser | null
): EmailLinkIdentity {
  if (!clerkUser) {
    return {
      eligible: false,
      emailHash: null,
      emailDisplay: null,
      primaryEmail: null,
      warning: null,
      emailCount: 0,
    };
  }

  const emailCount = clerkUser.emailAddresses.length;
  const primaryEmailAddress = clerkUser.primaryEmailAddress ?? null;
  const primaryEmail = primaryEmailAddress?.emailAddress ?? null;

  if (emailCount !== 1) {
    return {
      eligible: false,
      emailHash: null,
      emailDisplay: null,
      primaryEmail,
      warning:
        "Email-linked wallet features require exactly one email address on your Clerk account. Remove extra emails to continue.",
      emailCount,
    };
  }

  if (!primaryEmailAddress || !primaryEmail) {
    return {
      eligible: false,
      emailHash: null,
      emailDisplay: null,
      primaryEmail: null,
      warning:
        "Email-linked wallet features require one verified primary Clerk email.",
      emailCount,
    };
  }

  if (primaryEmailAddress.verification?.status !== "verified") {
    return {
      eligible: false,
      emailHash: null,
      emailDisplay: null,
      primaryEmail,
      warning:
        "Verify your primary Clerk email before using email-linked wallet features.",
      emailCount,
    };
  }

  const normalized = normalizeEmail(primaryEmail);
  return {
    eligible: true,
    emailHash: createHash("sha256").update(normalized).digest("hex"),
    emailDisplay: maskEmail(normalized),
    primaryEmail: normalized,
    warning: null,
    emailCount,
  };
}

export async function getCurrentEmailLinkIdentity(): Promise<EmailLinkIdentity> {
  const clerkUser = await currentUser();
  return getEmailLinkIdentityFromClerkUser(clerkUser);
}
