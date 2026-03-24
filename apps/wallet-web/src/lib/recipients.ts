// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import type { Bookmark } from "@/lib/api";

export type AddressRecipientShortcut = {
  id: string;
  name: string;
  recipientType: "address";
  address: string;
};

export type EmailRecipientShortcut = {
  id: string;
  name: string;
  recipientType: "email";
  emailHash: string;
  emailDisplay: string;
};

export type RecipientShortcut = AddressRecipientShortcut | EmailRecipientShortcut;

export function bookmarkToRecipientShortcut(bookmark: Bookmark): RecipientShortcut | null {
  if (bookmark.recipient_type === "email" && bookmark.email_hash && bookmark.email_display) {
    return {
      id: bookmark.id,
      name: bookmark.name,
      recipientType: "email",
      emailHash: bookmark.email_hash,
      emailDisplay: bookmark.email_display,
    };
  }

  if (bookmark.address) {
    return {
      id: bookmark.id,
      name: bookmark.name,
      recipientType: "address",
      address: bookmark.address,
    };
  }

  return null;
}

export function recipientMatchesQuery(recipient: RecipientShortcut, query: string): boolean {
  const normalizedQuery = query.trim().toLowerCase();
  if (!normalizedQuery) return true;

  const detail =
    recipient.recipientType === "address"
      ? recipient.address
      : recipient.emailDisplay;

  return (
    recipient.name.toLowerCase().includes(normalizedQuery) ||
    detail.toLowerCase().includes(normalizedQuery)
  );
}

export function recipientDisplayValue(recipient: RecipientShortcut): string {
  return recipient.recipientType === "address"
    ? recipient.address
    : recipient.emailDisplay;
}
