// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useEffect, useCallback } from "react";
import { ExternalLink } from "lucide-react";
import type { TransactionSummary, TransactionListResponse } from "@/lib/api";

interface TransactionListProps {
  walletId: string;
}

const USDC_FUJI_ADDRESS = "0x5425890298aed601595a70ab815c96711a31bc65";
const REUR_FUJI_ADDRESS = "0x76568bed5acf1a5cd888773c8cae9ea2a9131a63";

function tokenLabel(token: string): string {
  if (token === "native") return "AVAX";
  const normalized = token.toLowerCase();
  if (normalized === USDC_FUJI_ADDRESS) return "USDC";
  if (normalized === REUR_FUJI_ADDRESS) return "rEUR";
  return "TOKEN";
}

function truncateHash(hash: string): string {
  if (hash.length <= 18) return hash;
  return `${hash.slice(0, 10)}...${hash.slice(-6)}`;
}

function formatDate(timestamp: string): string {
  const date = new Date(timestamp);
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function statusClass(status: TransactionSummary["status"]) {
  if (status === "confirmed") return "status-chip success";
  if (status === "failed") return "status-chip failed";
  return "status-chip pending";
}

export function TransactionList({ walletId }: TransactionListProps) {
  const [transactions, setTransactions] = useState<TransactionSummary[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchTransactions = useCallback(async () => {
    setIsLoading(true);
    setError(null);

    try {
      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/transactions?network=fuji`,
        {
          method: "GET",
          credentials: "include",
        }
      );

      if (response.ok) {
        const data: TransactionListResponse = await response.json();
        setTransactions(data.transactions);
      } else if (response.status === 404) {
        setTransactions([]);
      } else {
        const text = await response.text();
        setError(text || `Failed to load transactions (${response.status})`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    } finally {
      setIsLoading(false);
    }
  }, [walletId]);

  useEffect(() => {
    void fetchTransactions();
  }, [fetchTransactions]);

  if (isLoading) {
    return (
      <article className="card pad">
        <div className="list-stack">
          <div className="skeleton-line" />
          <div className="skeleton-line" />
          <div className="skeleton-line" />
          <div className="skeleton-line" />
        </div>
      </article>
    );
  }

  if (error) {
    return (
      <article className="card pad">
        <div className="alert error">{error}</div>
        <div style={{ marginTop: "0.75rem" }}>
          <button onClick={() => void fetchTransactions()} className="btn btn-primary">
            Retry
          </button>
        </div>
      </article>
    );
  }

  if (transactions.length === 0) {
    return (
      <article className="card pad">
        <div className="alert success">No transactions yet. Send or receive funds to populate history.</div>
      </article>
    );
  }

  return (
    <article className="card pad">
      <div style={{ overflowX: "auto" }}>
        <table className="data-table" aria-label="Transactions">
          <thead>
            <tr>
              <th>Transaction</th>
              <th>Direction</th>
              <th>Amount</th>
              <th>Token</th>
              <th>Status</th>
              <th>Date</th>
            </tr>
          </thead>
          <tbody>
            {transactions.map((tx) => (
              <tr key={tx.tx_hash}>
                <td data-label="Transaction" style={{ position: "relative" }}>
                  <a
                    href={tx.explorer_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="tx-link"
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: "0.375rem" }}>
                      <span className="mono">{truncateHash(tx.tx_hash)}</span>
                      <ExternalLink size={12} style={{ flexShrink: 0, opacity: 0.5 }} />
                    </div>
                    <div className="helper-text">
                      {tx.direction === "sent" ? "To" : "From"}: {truncateHash(tx.direction === "sent" ? tx.to : tx.from)}
                    </div>
                  </a>
                </td>
                <td data-label="Direction">
                  <span className={`status-chip ${tx.direction === "sent" ? "warn" : "success"}`}>
                    {tx.direction === "sent" ? "Sent" : "Received"}
                  </span>
                </td>
                <td data-label="Amount" className="mono">{tx.direction === "sent" ? "-" : "+"}{tx.amount}</td>
                <td data-label="Token">{tokenLabel(tx.token)}</td>
                <td data-label="Status">
                  <span className={statusClass(tx.status)}>{tx.status}</span>
                </td>
                <td data-label="Date">{formatDate(tx.timestamp)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </article>
  );
}
