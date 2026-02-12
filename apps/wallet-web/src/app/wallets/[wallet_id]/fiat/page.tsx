// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";
import { FiatRequestPanel } from "./FiatRequestPanel";

interface FiatPageProps {
  params: Promise<{
    wallet_id: string;
  }>;
}

/**
 * Fiat request page.
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
    <SimpleWalletShell
      topBar={
        <>
          <div className="app-top-left">
            <a href={`/wallets/${wallet_id}`} className="btn btn-ghost">← Back</a>
            <span style={{ fontWeight: 700 }}>Fiat gateway</span>
          </div>
        </>
      }
    >
      {error ? <div className="alert alert-error">{error}</div> : wallet ? <FiatRequestPanel walletId={wallet.wallet_id} /> : null}
    </SimpleWalletShell>
  );
}
