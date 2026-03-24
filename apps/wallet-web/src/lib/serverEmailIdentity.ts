// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import "server-only";

import { createHash } from "node:crypto";
import { currentUser } from "@clerk/nextjs/server";
import { maskEmail, normalizeEmail } from "@/lib/emailHash";

export interface VerifiedEmailIdentity {
  emailHash: string;
  emailDisplay: string;
}

export async function getCurrentVerifiedEmailIdentity(): Promise<VerifiedEmailIdentity | null> {
  const clerkUser = await currentUser();
  if (!clerkUser) {
    return null;
  }

  const verifiedEmail =
    (clerkUser.primaryEmailAddress?.verification?.status === "verified"
      ? clerkUser.primaryEmailAddress.emailAddress
      : null) ??
    clerkUser.emailAddresses.find((email) => email.verification?.status === "verified")?.emailAddress ??
    null;

  if (!verifiedEmail) {
    return null;
  }

  const normalized = normalizeEmail(verifiedEmail);
  return {
    emailHash: createHash("sha256").update(normalized).digest("hex"),
    emailDisplay: maskEmail(normalized),
  };
}
