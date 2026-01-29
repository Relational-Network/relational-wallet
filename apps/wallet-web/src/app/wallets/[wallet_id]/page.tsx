// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { UserButton } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { WalletActions } from "@/components/WalletActions";
import { WalletBalance } from "@/components/WalletBalance";

interface WalletDetailPageProps {
  params: Promise<{
    wallet_id: string;
  }>;
}

/**
 * Wallet detail page (authenticated).
 *
 * Displays details for a specific wallet.
 */
export default async function WalletDetailPage({ params }: WalletDetailPageProps) {
  const { userId } = await auth();

  if (!userId) {
    redirect("/sign-in");
  }

  const { wallet_id } = await params;
  const token = await getSessionToken();

  // Fetch wallet details using the typed API client
  let wallet: WalletResponse | null = null;
  let error: string | null = null;

  const response = await apiClient.getWallet(token || "", wallet_id);

  if (response.success) {
    wallet = response.data;
  } else {
    // Handle different error statuses
    if (response.error.status === 401) {
      redirect("/sign-in");
    } else if (response.error.status === 403) {
      error = "Access denied. You do not have permission to view this wallet.";
    } else if (response.error.status === 404) {
      notFound();
    } else {
      error = `Unable to load wallet details: ${response.error.message}`;
    }
  }

  if (!wallet && !error) {
    notFound();
  }

  return (
    <main style={{ padding: "2rem", maxWidth: "800px", margin: "0 auto" }}>
      <header
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "2rem",
        }}
      >
        <div>
          <Link href="/wallets" style={{ color: "#666", textDecoration: "none" }}>
            ← Back to Wallets
          </Link>
          <h1 style={{ marginTop: "0.5rem" }}>{wallet?.label || "Wallet Details"}</h1>
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
        <div>
          <section
            style={{
              border: "1px solid #ddd",
              borderRadius: "4px",
              padding: "1.5rem",
              marginBottom: "2rem",
            }}
          >
            <h2 style={{ marginTop: 0 }}>Wallet Information</h2>

            <dl style={{ margin: 0 }}>
              <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
                Wallet ID
              </dt>
              <dd style={{ margin: "0.25rem 0 0 0", fontFamily: "monospace", color: "#333" }}>
                {wallet.wallet_id}
              </dd>

              <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
                Public Address
              </dt>
              <dd
                style={{
                  margin: "0.25rem 0 0 0",
                  fontFamily: "monospace",
                  wordBreak: "break-all",
                  backgroundColor: "#f5f5f5",
                  padding: "0.5rem",
                  borderRadius: "4px",
                  color: "#333",
                }}
              >
                {wallet.public_address}
              </dd>

              <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
                Label
              </dt>
              <dd style={{ margin: "0.25rem 0 0 0", color: "#333" }}>{wallet.label || "No label"}</dd>

              <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
                Status
              </dt>
              <dd style={{ margin: "0.25rem 0 0 0" }}>
                <span style={{
                  padding: "0.125rem 0.5rem",
                  borderRadius: "4px",
                  fontSize: "0.875rem",
                  fontWeight: "bold",
                  backgroundColor: wallet.status === "active" ? "#d4edda" : "#fff3cd",
                  color: wallet.status === "active" ? "#155724" : "#856404",
                }}>
                  {wallet.status}
                </span>
              </dd>

              <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
                Created
              </dt>
              <dd style={{ margin: "0.25rem 0 0 0", color: "#333" }}>
                {new Date(wallet.created_at).toLocaleString()}
              </dd>
            </dl>
          </section>

          {/* Wallet Balance Section - Client Component for real-time updates */}
          <WalletBalance
            walletId={wallet.wallet_id}
            publicAddress={wallet.public_address}
            walletStatus={wallet.status}
          />

          <section
            style={{
              border: "1px solid #ddd",
              borderRadius: "4px",
              padding: "1.5rem",
            }}
          >
            <h2 style={{ marginTop: 0 }}>Actions</h2>
            <WalletActions
              walletId={wallet.wallet_id}
              walletLabel={wallet.label ?? null}
              walletStatus={wallet.status}
            />
          </section>
        </div>
      ) : null}
    </main>
  );
}
