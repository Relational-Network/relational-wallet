// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useEffect, useCallback } from "react";

export interface TokenBalance {
  symbol: string;
  name: string;
  balance_raw: string;
  balance_formatted: string;
  decimals: number;
  contract_address: string | null;
}

export interface BalanceResponse {
  wallet_id: string;
  address: string;
  network: string;
  chain_id: number;
  native_balance: TokenBalance;
  token_balances: TokenBalance[];
}

interface WalletBalanceProps {
  walletId: string;
  publicAddress: string;
  walletStatus: string;
}

function formatBalance(value: string): string {
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) {
    return value;
  }
  return parsed.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: 6,
  });
}

export function WalletBalance({ walletId, publicAddress, walletStatus }: WalletBalanceProps) {
  const [balance, setBalance] = useState<BalanceResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const fetchBalance = useCallback(async () => {
    if (walletStatus === "deleted" || walletStatus === "suspended") {
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/balance?network=fuji`,
        {
          method: "GET",
          credentials: "include",
        }
      );

      if (response.ok) {
        const data: BalanceResponse = await response.json();
        setBalance(data);
        setLastUpdated(new Date());
      } else if (response.status === 503) {
        setError("Blockchain network unavailable. Please retry.");
      } else {
        const text = await response.text();
        setError(text || `Failed to fetch balance (${response.status})`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error fetching balance");
    } finally {
      setIsLoading(false);
    }
  }, [walletId, walletStatus]);

  useEffect(() => {
    void fetchBalance();
  }, [fetchBalance]);

  if (walletStatus === "deleted") {
    return null;
  }

  if (walletStatus === "suspended") {
    return <div className="alert warn">Balance queries are disabled for suspended wallets.</div>;
  }

  const usdcBalance = balance?.token_balances.find((token) => token.symbol.toUpperCase() === "USDC");
  const reurBalance = balance?.token_balances.find((token) => token.symbol.toUpperCase() === "REUR");

  return (
    <article className="card pad">
      <div className="section-header">
        <div>
          <h2 className="card-title">Balances</h2>
          <p className="card-subtitle">Source address: {publicAddress.slice(0, 10)}...{publicAddress.slice(-8)}</p>
        </div>
        <div className="inline-actions">
          <span className="pill">Fuji testnet</span>
          <button onClick={() => void fetchBalance()} disabled={isLoading} className="btn btn-ghost">
            {isLoading ? "Loading..." : "Refresh"}
          </button>
        </div>
      </div>

      {error ? <div className="alert error">{error}</div> : null}

      <div className="page-grid-2">
        <div className="token-tile">
          <span className="helper-text">AVAX (native)</span>
          <span className="token-value">
            {isLoading && !balance ? "..." : balance ? formatBalance(balance.native_balance.balance_formatted) : "-"}
          </span>
          <span className="mono">
            {isLoading && !balance ? "loading" : balance ? `${balance.native_balance.balance_raw} wei` : "no data"}
          </span>
        </div>

        <div className="token-tile">
          <span className="helper-text">USDC</span>
          <span className="token-value">
            {isLoading && !balance ? "..." : usdcBalance ? formatBalance(usdcBalance.balance_formatted) : "0"}
          </span>
          <span className="mono">{usdcBalance?.contract_address || "Not detected"}</span>
        </div>

        <div className="token-tile">
          <span className="helper-text">rEUR</span>
          <span className="token-value">
            {isLoading && !balance ? "..." : reurBalance ? formatBalance(reurBalance.balance_formatted) : "0"}
          </span>
          <span className="mono">{reurBalance?.contract_address || "Not detected"}</span>
        </div>
      </div>

      <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.75rem" }}>
        Last updated: {lastUpdated ? lastUpdated.toLocaleTimeString() : "not yet"}
      </p>
    </article>
  );
}
