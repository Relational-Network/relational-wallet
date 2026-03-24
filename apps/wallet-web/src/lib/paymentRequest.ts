// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

const ETH_ADDRESS_REGEX = /^0x[a-fA-F0-9]{40}$/;
const SUPPORTED_TOKENS = new Set(["native", "reur"]);
const NOTE_MAX_LENGTH = 140;

export interface PaymentRequestQuery {
  to?: string;
  amount?: string;
  token?: string;
  note?: string;
  /** Opaque payment-link token (takes precedence over to/amount/token/note). */
  ref?: string;
}

export interface ParsedPaymentRequest {
  to?: string;
  amount?: string;
  token: "native" | "reur";
  note?: string;
  /** If set, this was resolved from an opaque payment-link token. */
  ref?: string;
}

export interface PaymentRequestParseResult {
  prefill: ParsedPaymentRequest;
  warnings: string[];
}

function clean(value?: string): string | undefined {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
}

function isValidAmount(amount: string): boolean {
  const value = Number(amount);
  return Number.isFinite(value) && value > 0;
}

export function parsePaymentRequestQuery(
  query: PaymentRequestQuery
): PaymentRequestParseResult {
  const warnings: string[] = [];
  const prefill: ParsedPaymentRequest = {
    token: "native",
  };

  // If an opaque ref token is present, pass it through for server-side resolution
  const ref = clean(query.ref);
  if (ref) {
    prefill.ref = ref;
    return { prefill, warnings };
  }

  const to = clean(query.to);
  if (to) {
    if (ETH_ADDRESS_REGEX.test(to)) {
      prefill.to = to;
    } else {
      warnings.push("Ignored invalid recipient address from the payment link.");
    }
  }

  const amount = clean(query.amount);
  if (amount) {
    if (isValidAmount(amount)) {
      prefill.amount = amount;
    } else {
      warnings.push("Ignored invalid amount from the payment link.");
    }
  }

  const token = clean(query.token)?.toLowerCase();
  if (token) {
    if (SUPPORTED_TOKENS.has(token)) {
      prefill.token = token as "native" | "reur";
    } else {
      warnings.push("Unsupported token in payment link. Defaulted to AVAX.");
    }
  }

  const note = clean(query.note);
  if (note) {
    if (note.length > NOTE_MAX_LENGTH) {
      prefill.note = note.slice(0, NOTE_MAX_LENGTH);
      warnings.push(`Payment note exceeded ${NOTE_MAX_LENGTH} chars and was truncated.`);
    } else {
      prefill.note = note;
    }
  }

  return {
    prefill,
    warnings,
  };
}

export function buildPaymentRequestParams(
  request: ParsedPaymentRequest,
  options: { includeDefaultToken?: boolean } = {}
): URLSearchParams {
  const params = new URLSearchParams();

  if (request.to) params.set("to", request.to);
  if (request.amount) params.set("amount", request.amount);
  if (request.note) params.set("note", request.note);

  if (options.includeDefaultToken || request.token !== "native") {
    params.set("token", request.token);
  }

  return params;
}
