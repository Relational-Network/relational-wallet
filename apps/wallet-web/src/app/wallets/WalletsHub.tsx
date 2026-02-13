// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  FiatRequest,
  FiatRequestListResponse,
  TransactionListResponse,
  TransactionSummary,
  WalletListResponse,
  WalletResponse,
} from "@/lib/api";

const USDC_FUJI_ADDRESS = "0x5425890298aed601595a70ab815c96711a31bc65";
const REUR_FUJI_ADDRESS = "0x76568bed5acf1a5cd888773c8cae9ea2a9131a63";

function tokenLabel(token: string): string {
  if (token === "native") return "AVAX";
  const normalized = token.toLowerCase();
  if (normalized === USDC_FUJI_ADDRESS) return "USDC";
  if (normalized === REUR_FUJI_ADDRESS) return "rEUR";
  return "TOKEN";
}

function shortenAddress(address: string): string {
  if (address.length <= 18) {
    return address;
  }
  return `${address.slice(0, 8)}...${address.slice(-6)}`;
}

function sortByCreatedAtDesc(wallets: WalletResponse[]): WalletResponse[] {
  return [...wallets].sort((left, right) => {
    return new Date(right.created_at).getTime() - new Date(left.created_at).getTime();
  });
}

function formatTime(value: string): string {
  return new Date(value).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function WalletsHub() {
  const [wallets, setWallets] = useState<WalletResponse[]>([]);
  const [recentTransactions, setRecentTransactions] = useState<TransactionSummary[]>([]);
  const [pendingFiat, setPendingFiat] = useState<FiatRequest[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const hasWallets = wallets.length > 0;

  const fetchHubData = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      console.debug("[wallets.hub] loading hub modules");
      const walletsResponse = await fetch("/api/proxy/v1/wallets", {
        method: "GET",
        credentials: "include",
      });

      if (!walletsResponse.ok) {
        if (walletsResponse.status === 401) {
          setError("Session expired. Refresh and sign in again.");
          return;
        }
        const text = await walletsResponse.text();
        setError(text || `Failed to load wallets (${walletsResponse.status})`);
        return;
      }

      const walletPayload: WalletListResponse = await walletsResponse.json();
      const sortedWallets = sortByCreatedAtDesc(walletPayload.wallets);
      setWallets(sortedWallets);

      if (sortedWallets.length === 0) {
        setRecentTransactions([]);
        setPendingFiat([]);
        return;
      }

      const primaryWalletId = sortedWallets[0].wallet_id;

      const [transactionResponse, fiatResponse] = await Promise.all([
        fetch(
          `/api/proxy/v1/wallets/${encodeURIComponent(primaryWalletId)}/transactions?network=fuji`,
          {
            method: "GET",
            credentials: "include",
          }
        ),
        fetch(`/api/proxy/v1/fiat/requests?wallet_id=${encodeURIComponent(primaryWalletId)}`, {
          method: "GET",
          credentials: "include",
        }),
      ]);

      let loadedTransactions = 0;
      if (transactionResponse.ok) {
        const payload: TransactionListResponse = await transactionResponse.json();
        setRecentTransactions(payload.transactions.slice(0, 5));
        loadedTransactions = payload.transactions.length;
      } else {
        setRecentTransactions([]);
      }

      let loadedPendingFiat = 0;
      if (fiatResponse.ok) {
        const payload: FiatRequestListResponse = await fiatResponse.json();
        const pending = payload.requests.filter((request) =>
          request.status === "provider_pending" ||
          request.status === "awaiting_provider" ||
          request.status === "awaiting_user_deposit" ||
          request.status === "settlement_pending"
        );
        setPendingFiat(pending.slice(0, 5));
        loadedPendingFiat = pending.length;
      } else {
        setPendingFiat([]);
      }

      console.debug("[wallets.hub] modules loaded", {
        wallets: sortedWallets.length,
        recentTransactions: loadedTransactions,
        pendingFiat: loadedPendingFiat,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    void fetchHubData();
  }, [fetchHubData]);

  const stats = useMemo(
    () => [
      { label: "Wallets", value: String(wallets.length) },
      {
        label: "Active",
        value: String(wallets.filter((wallet) => wallet.status === "active").length),
      },
      {
        label: "Pending Fiat",
        value: String(pendingFiat.length),
      },
    ],
    [wallets, pendingFiat.length]
  );

  return (
    <section className="page-row">
      <div className="page-grid-2">
        <article className="card pad">
          <h2 className="card-title">Portfolio Snapshot</h2>
          <p className="card-subtitle">
            Unified overview for wallet inventory and operational risk surfaces.
          </p>
          <div className="page-grid-3" style={{ marginTop: "0.8rem" }}>
            {stats.map((stat) => (
              <div className="kpi" key={stat.label}>
                <div className="label">{stat.label}</div>
                <div className="value">{stat.value}</div>
              </div>
            ))}
          </div>
        </article>

        <article className="card pad">
          <h2 className="card-title">Quick Actions</h2>
          <p className="card-subtitle">Fast controls for common workflows.</p>
          <div className="inline-actions" style={{ marginTop: "0.86rem" }}>
            <Link className="btn btn-primary" href="/wallets/new">
              Create wallet
            </Link>
            <Link className="btn btn-soft" href={hasWallets ? `/wallets/${wallets[0].wallet_id}/send` : "/wallets"}>
              Send
            </Link>
            <Link className="btn btn-soft" href={hasWallets ? `/wallets/${wallets[0].wallet_id}/receive` : "/wallets"}>
              Receive
            </Link>
            <button className="btn btn-ghost" onClick={() => void fetchHubData()} disabled={isLoading}>
              {isLoading ? "Refreshing..." : "Refresh"}
            </button>
          </div>
          <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.72rem" }}>
            Debug logs remain enabled in browser console for data loading and wallet creation.
          </p>
        </article>
      </div>

      {error && <div className="alert error">{error}</div>}

      <div className="page-grid-2">
        <article className="card pad">
          <div className="section-header">
            <h2 className="card-title">Recent Activity</h2>
            <Link className="btn btn-ghost" href={hasWallets ? `/wallets/${wallets[0].wallet_id}/transactions` : "/wallets"}>
              View all
            </Link>
          </div>
          {isLoading ? (
            <div className="list-stack">
              <div className="skeleton-line" />
              <div className="skeleton-line" />
              <div className="skeleton-line" />
            </div>
          ) : recentTransactions.length > 0 ? (
            <div className="list-stack">
              {recentTransactions.map((transaction) => (
                <div key={transaction.tx_hash} className="card pad" style={{ boxShadow: "none" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "0.55rem", alignItems: "center" }}>
                    <div>
                      <div style={{ fontWeight: 600 }}>
                        {transaction.direction === "sent" ? "Sent" : "Received"} {transaction.amount} {tokenLabel(transaction.token)}
                      </div>
                      <div className="mono">{shortenAddress(transaction.tx_hash)}</div>
                    </div>
                    <span className={`status-chip ${transaction.status === "confirmed" ? "success" : transaction.status === "failed" ? "failed" : "pending"}`}>
                      {transaction.status}
                    </span>
                  </div>
                  <div className="helper-text" style={{ marginTop: "0.24rem" }}>{formatTime(transaction.timestamp)}</div>
                </div>
              ))}
            </div>
          ) : (
            <div className="alert success">No transactions yet. Once you send or receive, activity appears here.</div>
          )}
        </article>

        <article className="card pad">
          <div className="section-header">
            <h2 className="card-title">Pending Fiat Requests</h2>
            <Link className="btn btn-ghost" href={hasWallets ? `/wallets/${wallets[0].wallet_id}/fiat` : "/wallets"}>
              Open fiat
            </Link>
          </div>
          {isLoading ? (
            <div className="list-stack">
              <div className="skeleton-line" />
              <div className="skeleton-line" />
            </div>
          ) : pendingFiat.length > 0 ? (
            <div className="list-stack">
              {pendingFiat.map((request) => (
                <div key={request.request_id} className="card pad" style={{ boxShadow: "none" }}>
                  <div style={{ display: "flex", justifyContent: "space-between", gap: "0.5rem", alignItems: "center" }}>
                    <div>
                      <div style={{ fontWeight: 600 }}>
                        {request.direction === "on_ramp" ? "On-Ramp" : "Off-Ramp"} • EUR {request.amount_eur}
                      </div>
                      <div className="mono">{request.request_id}</div>
                    </div>
                    <span className="status-chip warn">pending</span>
                  </div>
                  <div className="helper-text" style={{ marginTop: "0.25rem" }}>{formatTime(request.created_at)}</div>
                </div>
              ))}
            </div>
          ) : (
            <div className="alert success">No pending fiat items right now.</div>
          )}
        </article>
      </div>

      <article className="card pad">
        <div className="section-header">
          <h2 className="card-title">Wallet Inventory</h2>
          {hasWallets ? <span className="pill good">{wallets.length} total</span> : <span className="pill warn">No wallets</span>}
        </div>

        {isLoading ? (
          <div className="wallet-grid">
            <div className="wallet-card"><div className="skeleton-line" /><div className="skeleton-line" /></div>
            <div className="wallet-card"><div className="skeleton-line" /><div className="skeleton-line" /></div>
          </div>
        ) : hasWallets ? (
          <div className="wallet-grid">
            {wallets.map((wallet) => (
              <Link
                className="wallet-card"
                href={`/wallets/${encodeURIComponent(wallet.wallet_id)}`}
                key={wallet.wallet_id}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: "0.6rem" }}>
                  <h3>{wallet.label || "Untitled wallet"}</h3>
                  <span className={`pill ${wallet.status === "active" ? "good" : "warn"}`}>
                    {wallet.status}
                  </span>
                </div>
                <div className="mono">{wallet.wallet_id}</div>
                <div className="mono">{shortenAddress(wallet.public_address)}</div>
                <div className="helper-text">Created {new Date(wallet.created_at).toLocaleDateString()}</div>
              </Link>
            ))}
          </div>
        ) : (
          <div className="alert success">
            No wallets yet. Create your first wallet to unlock send, receive, transaction history, and fiat rails.
          </div>
        )}
      </article>
    </section>
  );
}
