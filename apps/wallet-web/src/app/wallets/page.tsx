// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { UserButton } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { WalletList } from "@/components/WalletList";

/**
 * Wallets list page (authenticated).
 *
 * Displays all wallets for the current user.
 */
export default async function WalletsPage() {
  const { userId } = await auth();

  if (!userId) {
    redirect("/sign-in");
  }

  const token = await getSessionToken();

  // Fetch wallets using the typed API client
  let wallets: WalletResponse[] = [];
  let error: string | null = null;

  if (token) {
    const response = await apiClient.listWallets(token);
    if (response.success) {
      wallets = response.data.wallets;
    } else {
      // Handle different error statuses
      // 401 can happen during hot reload - show a softer message with refresh hint
      if (response.error.status === 401) {
        error = "Session expired or invalid. Please refresh the page or sign out and back in.";
      } else if (response.error.status === 403) {
        error = "Access denied. You do not have permission to view wallets.";
      } else if (response.error.message.includes("ECONNREFUSED")) {
        error = "Cannot connect to backend. Make sure the enclave server is running.";
      } else {
        error = `Unable to load wallets: ${response.error.message}`;
      }
    }
  } else {
    error = "Authentication token not available. Please try signing out and back in.";
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
        <h1 style={{ margin: 0 }}>My Wallets</h1>
        <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
          <Link href="/account">Account</Link>
          <UserButton />
        </div>
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
          <p style={{ margin: 0 }}>{error}</p>
          <Link
            href="/wallets"
            style={{
              display: "inline-block",
              marginTop: "0.75rem",
              padding: "0.5rem 1rem",
              backgroundColor: "#c00",
              color: "#fff",
              textDecoration: "none",
              borderRadius: "4px",
            }}
          >
            Refresh Page
          </Link>
        </div>
      ) : (
        <>
          <div style={{ marginBottom: "1rem" }}>
            <Link
              href="/wallets/new"
              style={{
                display: "inline-block",
                padding: "0.5rem 1rem",
                backgroundColor: "#333",
                color: "#fff",
                textDecoration: "none",
                borderRadius: "4px",
              }}
            >
              Create Wallet
            </Link>
          </div>

          <WalletList wallets={wallets} />
        </>
      )}
    </main>
  );
}
