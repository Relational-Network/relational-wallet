// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { UserButton } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import {
  buildPaymentRequestParams,
  parsePaymentRequestQuery,
} from "@/lib/paymentRequest";

interface PayPageProps {
  searchParams: Promise<{
    to?: string;
    amount?: string;
    token?: string;
    note?: string;
  }>;
}

/**
 * Payment request landing page.
 *
 * Users can open a shared `/pay` link and choose one of their own wallets
 * before being redirected to the prefilled send form.
 */
export default async function PayPage({ searchParams }: PayPageProps) {
  const { userId } = await auth();
  if (!userId) {
    redirect("/sign-in");
  }

  const query = await searchParams;
  const parsedRequest = parsePaymentRequestQuery(query);
  const token = await getSessionToken();

  let wallets: WalletResponse[] = [];
  let error: string | null = null;

  if (token) {
    const response = await apiClient.listWallets(token);
    if (response.success) {
      wallets = response.data.wallets.filter((w) => w.status === "active");
    } else if (response.error.status === 401) {
      redirect("/sign-in");
    } else if (response.error.status === 403) {
      error = "Access denied. Unable to load your wallets.";
    } else {
      error = `Unable to load wallets: ${response.error.message}`;
    }
  } else {
    error = "Authentication token not available.";
  }

  const requestParams = buildPaymentRequestParams(parsedRequest.prefill, {
    includeDefaultToken: true,
  });
  const requestSuffix = requestParams.toString();

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
          <Link href="/wallets" style={{ color: "#666", textDecoration: "none" }}>
            ← Back to Wallets
          </Link>
          <h1 style={{ marginTop: "0.5rem" }}>Pay Request</h1>
        </div>
        <UserButton />
      </header>

      <section
        style={{
          border: "1px solid #ddd",
          borderRadius: "8px",
          padding: "1rem",
          marginBottom: "1rem",
          backgroundColor: "#f8f9fa",
        }}
      >
        <p style={{ margin: 0, color: "#333" }}>
          Choose which of your wallets should send this payment request.
        </p>
        <div style={{ marginTop: "0.5rem", fontSize: "0.875rem", color: "#666" }}>
          {parsedRequest.prefill.to ? (
            <div>To: {parsedRequest.prefill.to}</div>
          ) : (
            <div>To: not specified</div>
          )}
          {parsedRequest.prefill.amount ? (
            <div>Amount: {parsedRequest.prefill.amount}</div>
          ) : (
            <div>Amount: open</div>
          )}
          <div>Token: {parsedRequest.prefill.token === "usdc" ? "USDC" : "AVAX"}</div>
          {parsedRequest.prefill.note ? <div>Note: {parsedRequest.prefill.note}</div> : null}
        </div>
      </section>

      {parsedRequest.warnings.length > 0 ? (
        <section
          style={{
            padding: "0.875rem",
            border: "1px solid #ffe08a",
            borderRadius: "8px",
            marginBottom: "1rem",
            backgroundColor: "#fffbeb",
            color: "#7c5800",
          }}
        >
          <p style={{ margin: 0, fontWeight: "bold", fontSize: "0.9375rem" }}>
            Some link details were adjusted
          </p>
          <ul style={{ margin: "0.5rem 0 0 1.25rem", padding: 0 }}>
            {parsedRequest.warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </section>
      ) : null}

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
      ) : wallets.length === 0 ? (
        <div
          style={{
            padding: "1rem",
            border: "1px solid #ddd",
            borderRadius: "4px",
            color: "#666",
            backgroundColor: "#fff",
          }}
        >
          No active wallets available. Create one first.
        </div>
      ) : (
        <div style={{ display: "grid", gap: "0.75rem" }}>
          {wallets.map((wallet) => {
            const href = `/wallets/${encodeURIComponent(wallet.wallet_id)}/send${requestSuffix ? `?${requestSuffix}` : ""}`;
            return (
              <Link
                key={wallet.wallet_id}
                href={href}
                style={{
                  border: "1px solid #ddd",
                  borderRadius: "8px",
                  padding: "1rem",
                  textDecoration: "none",
                  color: "inherit",
                  backgroundColor: "#fff",
                }}
              >
                <div style={{ fontWeight: "bold" }}>{wallet.label || "Wallet"}</div>
                <div style={{ fontFamily: "monospace", fontSize: "0.8125rem", color: "#666", marginTop: "0.25rem" }}>
                  {wallet.public_address}
                </div>
                <div style={{ marginTop: "0.5rem", color: "#007bff", fontWeight: "bold" }}>
                  Use This Wallet →
                </div>
              </Link>
            );
          })}
        </div>
      )}
    </main>
  );
}
