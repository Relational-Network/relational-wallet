// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { SignedIn, SignedOut, UserButton } from "@clerk/nextjs";
import { redirect } from "next/navigation";
import { auth } from "@clerk/nextjs/server";
import { apiClient, type ReadyResponse } from "@/lib/api";

/**
 * Landing page for Relational Wallet.
 *
 * - Signed out: Shows sign-in/sign-up links and backend status
 * - Signed in: Redirects to /wallets
 */
export default async function HomePage() {
  const { userId } = await auth();

  // Redirect authenticated users to wallets page
  if (userId) {
    redirect("/wallets");
  }

  // Check backend health status
  let backendStatus: { connected: boolean; data?: ReadyResponse; error?: string } = {
    connected: false,
  };

  const healthResponse = await apiClient.checkHealth();
  if (healthResponse.success) {
    backendStatus = { connected: true, data: healthResponse.data };
  } else {
    backendStatus = { connected: false, error: healthResponse.error.message };
  }

  return (
    <main style={{ padding: "2rem", maxWidth: "600px", margin: "0 auto" }}>
      <h1>Relational Wallet</h1>
      <p>Custodial Avalanche wallet service secured by Intel SGX.</p>

      {/* Backend Status */}
      <div
        style={{
          marginTop: "1.5rem",
          padding: "1rem",
          border: "1px solid",
          borderColor: backendStatus.connected ? "#4caf50" : "#f44336",
          borderRadius: "4px",
          backgroundColor: backendStatus.connected ? "#e8f5e9" : "#ffebee",
        }}
      >
        <strong>Enclave Backend:</strong>{" "}
        {backendStatus.connected ? (
          <span style={{ color: "#2e7d32" }}>
            ✓ Connected ({backendStatus.data?.status})
          </span>
        ) : (
          <span style={{ color: "#c62828" }}>
            ✗ Offline {backendStatus.error && `(${backendStatus.error})`}
          </span>
        )}
        {backendStatus.connected && backendStatus.data?.checks && (
          <div style={{ marginTop: "0.5rem", fontSize: "0.875rem", color: "#666" }}>
            Service: {backendStatus.data.checks.service}
            {backendStatus.data.checks.data_dir && (
              <>, Data Dir: {backendStatus.data.checks.data_dir}</>
            )}
          </div>
        )}
      </div>

      <SignedOut>
        <div style={{ marginTop: "2rem" }}>
          <p>Please sign in to access your wallets.</p>
          <nav style={{ display: "flex", gap: "1rem", marginTop: "1rem" }}>
            <Link
              href="/sign-in"
              style={{
                padding: "0.5rem 1rem",
                border: "1px solid #333",
                textDecoration: "none",
                color: "#333",
              }}
            >
              Sign In
            </Link>
            <Link
              href="/sign-up"
              style={{
                padding: "0.5rem 1rem",
                border: "1px solid #333",
                backgroundColor: "#333",
                color: "#fff",
                textDecoration: "none",
              }}
            >
              Sign Up
            </Link>
          </nav>
        </div>
      </SignedOut>

      <SignedIn>
        <div style={{ marginTop: "2rem" }}>
          <p>Welcome back!</p>
          <div style={{ display: "flex", alignItems: "center", gap: "1rem" }}>
            <UserButton />
            <Link href="/wallets">Go to Wallets</Link>
          </div>
        </div>
      </SignedIn>
    </main>
  );
}
