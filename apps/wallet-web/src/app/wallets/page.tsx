// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { auth } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { SimpleWalletDashboard } from "./SimpleWalletDashboard";
import { getCurrentEmailLinkIdentity } from "@/lib/serverEmailIdentity";
import type {
  BalanceResponse,
  TransactionListResponse,
  WalletListResponse,
  WalletResponse,
} from "@/lib/api";

const BACKEND_URL = process.env.WALLET_API_BASE_URL || "https://localhost:8080";

/**
 * Direct backend fetch (bypasses the proxy route handler).
 * Used during SSR to prefetch data so the dashboard renders instantly
 * instead of showing a blank shell while client-side fetches complete.
 */
async function backendGet<T>(path: string, token: string): Promise<T | null> {
  try {
    const res = await fetch(`${BACKEND_URL}${path}`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${token}`,
      },
      cache: "no-store",
    });
    if (!res.ok) return null;
    return (await res.json()) as T;
  } catch {
    return null;
  }
}

export default async function WalletsPage() {
  const { userId, getToken, sessionClaims } = await auth();
  if (!userId) redirect("/sign-in");

  const isAdmin =
    (sessionClaims?.publicMetadata as { role?: string } | undefined)?.role === "admin";
  const emailLinkIdentity = await getCurrentEmailLinkIdentity();

  const token =
    (await getToken({ template: "default" }).catch(() => null)) ??
    (await getToken());

  if (!token) {
    // Can't prefetch without a token — fall back to client-side loading.
    return (
      <SimpleWalletDashboard
        isAdmin={isAdmin}
        verifiedEmailHash={emailLinkIdentity.emailHash}
        verifiedEmailDisplay={emailLinkIdentity.emailDisplay}
        emailLinkWarning={emailLinkIdentity.warning}
      />
    );
  }

  // Prefetch wallets during SSR
  const walletsPayload = await backendGet<WalletListResponse>(
    "/v1/wallets",
    token,
  );
  const wallets: WalletResponse[] = walletsPayload?.wallets ?? [];

  // Select the first active wallet (same heuristic as the client dashboard)
  const firstWallet =
    wallets.find((w) => w.status === "active") ?? wallets[0] ?? null;

  // Prefetch balance + transactions for the selected wallet in parallel
  let initialBalance: BalanceResponse | null = null;
  let initialTransactions: TransactionListResponse | null = null;

  if (firstWallet) {
    const wid = encodeURIComponent(firstWallet.wallet_id);
    [initialBalance, initialTransactions] = await Promise.all([
      backendGet<BalanceResponse>(
        `/v1/wallets/${wid}/balance?network=fuji`,
        token,
      ),
      backendGet<TransactionListResponse>(
        `/v1/wallets/${wid}/transactions?network=fuji&limit=5`,
        token,
      ),
    ]);
  }

  return (
    <SimpleWalletDashboard
      initialWallets={wallets}
      initialSelectedWalletId={firstWallet?.wallet_id ?? null}
      initialBalance={initialBalance}
      initialTransactions={initialTransactions}
      isAdmin={isAdmin}
      verifiedEmailHash={emailLinkIdentity.emailHash}
      verifiedEmailDisplay={emailLinkIdentity.emailDisplay}
      emailLinkWarning={emailLinkIdentity.warning}
    />
  );
}
