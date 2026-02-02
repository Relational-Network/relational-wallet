// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { UserButton } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { SendForm } from "./SendForm";

interface SendPageProps {
  params: Promise<{
    wallet_id: string;
  }>;
}

/**
 * Send transaction page.
 * Server component that loads wallet data and renders the SendForm client component.
 */
export default async function SendPage({ params }: SendPageProps) {
  const { userId } = await auth();

  if (!userId) {
    redirect("/sign-in");
  }

  const { wallet_id } = await params;
  const token = await getSessionToken();

  // Fetch wallet details
  let wallet: WalletResponse | null = null;
  let error: string | null = null;

  const response = await apiClient.getWallet(token || "", wallet_id);

  if (response.success) {
    wallet = response.data;
  } else {
    if (response.error.status === 401) {
      redirect("/sign-in");
    } else if (response.error.status === 403) {
      error = "Access denied. You do not have permission to use this wallet.";
    } else if (response.error.status === 404) {
      notFound();
    } else {
      error = `Unable to load wallet: ${response.error.message}`;
    }
  }

  if (!wallet && !error) {
    notFound();
  }

  // Check wallet status
  if (wallet && wallet.status !== "active") {
    error = `Cannot send from a ${wallet.status} wallet.`;
  }

  return (
    <main style={{ padding: "2rem", maxWidth: "600px", margin: "0 auto" }}>
      <header
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "2rem",
        }}
      >
        <div>
          <Link
            href={`/wallets/${wallet_id}`}
            style={{ color: "#666", textDecoration: "none" }}
          >
            ← Back to Wallet
          </Link>
          <h1 style={{ marginTop: "0.5rem" }}>Send Transaction</h1>
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
        <SendForm
          walletId={wallet.wallet_id}
          publicAddress={wallet.public_address}
          walletLabel={wallet.label ?? null}
        />
      ) : null}
    </main>
  );
}
