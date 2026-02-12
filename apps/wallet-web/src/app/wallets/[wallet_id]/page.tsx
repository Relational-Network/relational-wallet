// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";
import { WalletActions } from "@/components/WalletActions";
import { WalletBalance } from "@/components/WalletBalance";
import { CopyAddress } from "@/components/CopyAddress";
import { AddressQRCode } from "@/components/AddressQRCode";

interface WalletDetailPageProps {
  params: Promise<{ wallet_id: string }>;
}

export default async function WalletDetailPage({ params }: WalletDetailPageProps) {
  const { userId } = await auth();
  if (!userId) redirect("/sign-in");

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

  if (!wallet && !error) notFound();

  return (
    <SimpleWalletShell
      topBar={
        <>
          <div className="app-top-left">
            <Link href="/wallets" className="btn btn-ghost">← Back</Link>
            <span style={{ fontWeight: 700 }}>{wallet?.label || "Wallet"}</span>
          </div>
          <div className="app-top-right">
            {wallet && (
              <>
                <Link className="btn btn-primary" href={`/wallets/${wallet.wallet_id}/send`}>Send</Link>
                <Link className="btn btn-secondary" href={`/wallets/${wallet.wallet_id}/receive`}>Receive</Link>
              </>
            )}
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
                <span className={`badge ${wallet.status === "active" ? "badge-success" : "badge-warning"}`}>
                  {wallet.status}
                </span>
                <div>
                  <div className="text-muted">Wallet ID</div>
                  <div style={{ fontFamily: "var(--font-mono)", fontSize: "0.8125rem" }}>{wallet.wallet_id}</div>
                </div>
                <CopyAddress address={wallet.public_address} />
                <div className="text-muted">Created {new Date(wallet.created_at).toLocaleString()}</div>
              </div>
              <AddressQRCode address={wallet.public_address} size={126} />
            </div>
          </div>

          <WalletBalance
            walletId={wallet.wallet_id}
            publicAddress={wallet.public_address}
            walletStatus={wallet.status}
          />

          <div className="card card-pad">
            <h3 className="section-title">Wallet actions</h3>
            <p className="text-secondary" style={{ margin: "0.25rem 0 0.75rem" }}>Destructive actions require confirmation.</p>
            <WalletActions
              walletId={wallet.wallet_id}
              walletLabel={wallet.label ?? null}
              walletStatus={wallet.status}
            />
          </div>
        </div>
      ) : null}
    </SimpleWalletShell>
  );
}
