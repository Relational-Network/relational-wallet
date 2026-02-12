// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { auth } from "@clerk/nextjs/server";
import { redirect, notFound } from "next/navigation";
import { apiClient, type Bookmark, type WalletResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { parsePaymentRequestQuery } from "@/lib/paymentRequest";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";
import { SendForm } from "./SendForm";

interface SendPageProps {
  params: Promise<{
    wallet_id: string;
  }>;
  searchParams: Promise<{
    to?: string;
    amount?: string;
    token?: string;
    note?: string;
  }>;
}

/**
 * Send transaction page.
 */
export default async function SendPage({ params, searchParams }: SendPageProps) {
  const { userId } = await auth();

  if (!userId) {
    redirect("/sign-in");
  }

  const { wallet_id } = await params;
  const query = await searchParams;
  const parsedRequest = parsePaymentRequestQuery(query);
  const token = await getSessionToken();

  let wallet: WalletResponse | null = null;
  let bookmarks: Bookmark[] = [];
  let error: string | null = null;
  let shortcutError: string | null = null;

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

  if (wallet && wallet.status !== "active") {
    error = `Cannot send from a ${wallet.status} wallet.`;
  }

  if (!error && token) {
    const bookmarksResponse = await apiClient.listBookmarks(token, wallet_id);
    if (bookmarksResponse.success) {
      bookmarks = bookmarksResponse.data;
    } else if (bookmarksResponse.error.status === 401) {
      redirect("/sign-in");
    } else if (bookmarksResponse.error.status !== 404) {
      shortcutError = `Saved recipients unavailable: ${bookmarksResponse.error.message}`;
    }
  }

  return (
    <SimpleWalletShell
      topBar={
        <>
          <div className="app-top-left">
            <a href={`/wallets/${wallet_id}`} className="btn btn-ghost">← Back</a>
            <span style={{ fontWeight: 700 }}>Send</span>
          </div>
        </>
      }
    >
      {error ? (
        <div className="alert alert-error">{error}</div>
      ) : wallet ? (
        <SendForm
          walletId={wallet.wallet_id}
          publicAddress={wallet.public_address}
          walletLabel={wallet.label ?? null}
          prefill={parsedRequest.prefill}
          prefillWarnings={parsedRequest.warnings}
          shortcuts={bookmarks.map((bookmark) => ({
            id: bookmark.id,
            name: bookmark.name,
            address: bookmark.address,
          }))}
          shortcutsLoadError={shortcutError}
        />
      ) : null}
    </SimpleWalletShell>
  );
}
