// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { UserButton } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { FiatRequestPanel } from "./FiatRequestPanel";

interface FiatPageProps {
  params: Promise<{
    wallet_id: string;
  }>;
}

/**
 * Fiat request page (on-ramp/off-ramp).
 */
export default async function FiatPage({ params }: FiatPageProps) {
  const { userId } = await auth();
  if (!userId) {
    redirect("/sign-in");
  }

  const { wallet_id } = await params;
  const token = await getSessionToken();

  let wallet: WalletResponse | null = null;
  let error: string | null = null;

  const response = await apiClient.getWallet(token || "", wallet_id);
  if (response.success) {
    wallet = response.data;
  } else if (response.error.status === 401) {
    redirect("/sign-in");
  } else if (response.error.status === 403) {
    error = "Access denied. You do not have permission to use this wallet.";
  } else if (response.error.status === 404) {
    notFound();
  } else {
    error = `Unable to load wallet: ${response.error.message}`;
  }

  if (!wallet && !error) {
    notFound();
  }

  return (
    <main style={{ padding: "2rem", maxWidth: "760px", margin: "0 auto" }}>
      <header
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "1.5rem",
        }}
      >
        <div>
          <Link
            href={`/wallets/${wallet_id}`}
            style={{ color: "#666", textDecoration: "none" }}
          >
            ← Back to Wallet
          </Link>
          <h1 style={{ marginTop: "0.5rem" }}>Fiat On/Off-Ramp</h1>
          {wallet ? (
            <p style={{ margin: 0, color: "#666" }}>
              {wallet.label || "Wallet"} • {wallet.public_address.slice(0, 8)}...
              {wallet.public_address.slice(-6)}
            </p>
          ) : null}
        </div>
        <UserButton />
      </header>

      {error ? (
        <div
          style={{
            padding: "1rem",
            backgroundColor: "#fee",
            border: "1px solid #f00",
            borderRadius: "4px",
            color: "#c00",
          }}
        >
          {error}
        </div>
      ) : wallet ? (
        <FiatRequestPanel walletId={wallet.wallet_id} />
      ) : null}
    </main>
  );
}
