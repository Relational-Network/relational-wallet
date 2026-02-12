// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { UserButton } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { CopyAddress } from "@/components/CopyAddress";
import { AddressQRCode } from "@/components/AddressQRCode";
// import { CashExchangeGuide } from "@/components/CashExchangeGuide";
import { PaymentRequestBuilder } from "./PaymentRequestBuilder";

interface ReceivePageProps {
  params: Promise<{
    wallet_id: string;
  }>;
}

/**
 * Receive page — shows wallet address + QR code for receiving funds.
 *
 * Supports the Cash Exchange (P2P) and Pull Transfer user stories
 * by giving users an easy way to share their address.
 */
export default async function ReceivePage({ params }: ReceivePageProps) {
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
  } else {
    if (response.error.status === 401) {
      redirect("/sign-in");
    } else if (response.error.status === 403) {
      error = "Access denied.";
    } else if (response.error.status === 404) {
      notFound();
    } else {
      error = `Unable to load wallet: ${response.error.message}`;
    }
  }

  if (!wallet && !error) {
    notFound();
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
          <h1 style={{ marginTop: "0.5rem" }}>Receive Funds</h1>
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
          {/* <CashExchangeGuide walletId={wallet.wallet_id} currentStep="receive" /> */}

          {/* QR Code */}
          <section
            style={{
              border: "1px solid #ddd",
              borderRadius: "4px",
              padding: "2rem",
              marginBottom: "1.5rem",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              gap: "1rem",
            }}
          >
            <h2 style={{ margin: 0 }}>{wallet.label || "My Wallet"}</h2>
            <AddressQRCode address={wallet.public_address} size={200} />
          </section>

          {/* Address + Copy */}
          <section
            style={{
              border: "1px solid #ddd",
              borderRadius: "4px",
              padding: "1.5rem",
              marginBottom: "1.5rem",
            }}
          >
            <h3 style={{ marginTop: 0, color: "#666" }}>Public Address</h3>
            <p
              style={{
                fontFamily: "monospace",
                wordBreak: "break-all",
                backgroundColor: "#f5f5f5",
                padding: "0.75rem",
                borderRadius: "4px",
                margin: "0 0 0.75rem 0",
                fontSize: "0.9375rem",
                color: "#333",
              }}
            >
              {wallet.public_address}
            </p>
            <CopyAddress address={wallet.public_address} />
          </section>

          <PaymentRequestBuilder
            recipientAddress={wallet.public_address}
          />

          {/* Network Info */}
          <section
            style={{
              border: "1px solid #ddd",
              borderRadius: "4px",
              padding: "1.5rem",
              marginBottom: "1.5rem",
            }}
          >
            <h3 style={{ marginTop: 0, color: "#666" }}>Network</h3>
            <p style={{ margin: 0, color: "#333" }}>
              Avalanche C-Chain (Fuji Testnet)
            </p>
            <p style={{ margin: "0.25rem 0 0 0", fontSize: "0.8125rem", color: "#999" }}>
              Only send AVAX or supported tokens on the Avalanche C-Chain to
              this address.
            </p>
          </section>

          {/* Faucet Links */}
          <section
            style={{
              border: "1px solid #ddd",
              borderRadius: "4px",
              padding: "1.5rem",
              marginBottom: "1.5rem",
            }}
          >
            <h3 style={{ marginTop: 0, color: "#666" }}>Testnet Faucets</h3>
            <p
              style={{
                margin: "0 0 0.75rem 0",
                fontSize: "0.875rem",
                color: "#666",
              }}
            >
              Get free testnet tokens for development:
            </p>
            <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
              <a
                href="https://core.app/tools/testnet-faucet/?subnet=c&token=c"
                target="_blank"
                rel="noopener noreferrer"
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "0.375rem",
                  padding: "0.5rem 1rem",
                  border: "1px solid #ccc",
                  borderRadius: "4px",
                  color: "#333",
                  textDecoration: "none",
                  fontSize: "0.875rem",
                }}
              >
                🔺 AVAX Faucet
              </a>
              <a
                href="https://faucet.circle.com/"
                target="_blank"
                rel="noopener noreferrer"
                style={{
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "0.375rem",
                  padding: "0.5rem 1rem",
                  border: "1px solid #ccc",
                  borderRadius: "4px",
                  color: "#333",
                  textDecoration: "none",
                  fontSize: "0.875rem",
                }}
              >
                💲 USDC Faucet (Circle)
              </a>
            </div>
          </section>

          {/* Actions */}
          <div style={{ display: "flex", gap: "1rem" }}>
            <Link
              href={`/wallets/${wallet.wallet_id}`}
              style={{
                flex: 1,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                padding: "0.75rem",
                border: "1px solid #ccc",
                borderRadius: "4px",
                color: "#333",
                textDecoration: "none",
                fontWeight: "bold",
              }}
            >
              ← Wallet Details
            </Link>
            <Link
              href={`/wallets/${wallet.wallet_id}/send`}
              style={{
                flex: 1,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                padding: "0.75rem",
                backgroundColor: "#007bff",
                color: "white",
                textDecoration: "none",
                borderRadius: "4px",
                fontWeight: "bold",
              }}
            >
              ↗ Send
            </Link>
          </div>
        </div>
      ) : null}
    </main>
  );
}
