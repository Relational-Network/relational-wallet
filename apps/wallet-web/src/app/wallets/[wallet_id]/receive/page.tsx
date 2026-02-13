// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";
import { CopyAddress } from "@/components/CopyAddress";
import { AddressQRCode } from "@/components/AddressQRCode";
import { PaymentRequestBuilder } from "./PaymentRequestBuilder";

interface ReceivePageProps {
  params: Promise<{
    wallet_id: string;
  }>;
}

/**
 * Receive page for sharing wallet address and pull request links.
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
  } else if (response.error.status === 401) {
    redirect("/sign-in");
  } else if (response.error.status === 403) {
    error = "Access denied.";
  } else if (response.error.status === 404) {
    notFound();
  } else {
    error = `Unable to load wallet: ${response.error.message}`;
  }

  if (!wallet && !error) {
    notFound();
  }

  return (
    <SimpleWalletShell
      topBar={
        <>
          <div className="app-top-left">
            <Link href={`/wallets/${wallet_id}`} className="btn btn-ghost">← Back</Link>
            <span style={{ fontWeight: 700 }}>Receive</span>
          </div>
        </>
      }
    >
      {error ? (
        <div className="alert alert-error">{error}</div>
      ) : wallet ? (
        <div className="stack">
          <div className="card card-pad">
            <div className="grid-2">
              <div className="stack-sm">
                <h3 className="section-title">{wallet.label || "Wallet"}</h3>
                <p className="text-muted" style={{ margin: 0 }}>Avalanche C-Chain · Fuji testnet</p>
                <CopyAddress address={wallet.public_address} />
              </div>
              <AddressQRCode address={wallet.public_address} size={180} />
            </div>
          </div>

          <PaymentRequestBuilder recipientAddress={wallet.public_address} />

          <div className="card card-pad">
            <h3 className="section-title">Testnet faucets</h3>
            <p className="text-secondary" style={{ margin: "0.25rem 0 0.75rem" }}>Get AVAX and USDC test funds.</p>
            <div className="row" style={{ gap: "0.5rem", flexWrap: "wrap" }}>
              <a className="btn btn-secondary" href="https://core.app/tools/testnet-faucet/?subnet=c&token=c" target="_blank" rel="noopener noreferrer">AVAX faucet</a>
              <a className="btn btn-secondary" href="https://faucet.circle.com/" target="_blank" rel="noopener noreferrer">USDC faucet</a>
            </div>
          </div>
        </div>
      ) : null}
    </SimpleWalletShell>
  );
}
