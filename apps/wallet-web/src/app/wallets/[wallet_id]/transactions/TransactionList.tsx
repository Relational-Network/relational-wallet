// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useEffect, useCallback } from "react";
import type { TransactionSummary, TransactionListResponse } from "@/lib/api";

interface TransactionListProps {
  walletId: string;
}

/**
 * Status badge component with fixed dimensions.
 */
function StatusBadge({ status }: { status: "pending" | "confirmed" | "failed" }) {
  const styles: Record<string, { bg: string; color: string; text: string }> = {
    pending: { bg: "#fff3cd", color: "#856404", text: "Pending" },
    confirmed: { bg: "#d4edda", color: "#155724", text: "Confirmed" },
    failed: { bg: "#f8d7da", color: "#721c24", text: "Failed" },
  };

  const { bg, color, text } = styles[status];

  return (
    <span
      style={{
        display: "inline-block",
        width: "80px",
        textAlign: "center",
        padding: "0.25rem 0.5rem",
        borderRadius: "4px",
        fontSize: "0.75rem",
        fontWeight: "bold",
        backgroundColor: bg,
        color,
      }}
    >
      {text}
    </span>
  );
}

/**
 * Truncate transaction hash for display.
 */
function truncateHash(hash: string): string {
  if (hash.length <= 18) return hash;
  return `${hash.slice(0, 10)}...${hash.slice(-6)}`;
}

/**
 * Format timestamp for display.
 */
function formatDate(timestamp: string): string {
  const date = new Date(timestamp);
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Transaction list component with fixed-size rows.
 * Fetches and displays transaction history for a wallet.
 */
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
    fetchTransactions();
  }, [fetchTransactions]);

  if (isLoading) {
    return (
      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: "8px",
          padding: "2rem",
          minHeight: "400px",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "center",
            alignItems: "center",
            height: "200px",
            color: "#666",
          }}
        >
          Loading transactions...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: "8px",
          padding: "2rem",
          minHeight: "400px",
        }}
      >
        <div
          style={{
            padding: "1rem",
            backgroundColor: "#fee",
            border: "1px solid #f00",
            borderRadius: "4px",
            color: "#c00",
            marginBottom: "1rem",
          }}
        >
          {error}
        </div>
        <button
          onClick={fetchTransactions}
          style={{
            padding: "0.5rem 1rem",
            backgroundColor: "#007bff",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: "pointer",
          }}
        >
          Retry
        </button>
      </div>
    );
  }

  if (transactions.length === 0) {
    return (
      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: "8px",
          padding: "2rem",
          minHeight: "400px",
          textAlign: "center",
        }}
      >
        <div style={{ color: "#666", marginTop: "3rem" }}>
          <p style={{ fontSize: "1.25rem" }}>No transactions yet</p>
          <p>Transactions you send or receive will appear here.</p>
        </div>
      </div>
    );
  }

  return (
    <div
      style={{
        border: "1px solid #ddd",
        borderRadius: "8px",
        overflow: "hidden",
        minHeight: "400px",
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 80px 100px 100px 80px 120px",
          gap: "0.5rem",
          padding: "1rem",
          backgroundColor: "#f8f9fa",
          borderBottom: "1px solid #ddd",
          fontWeight: "bold",
          fontSize: "0.875rem",
          color: "#666",
        }}
      >
        <div>Transaction</div>
        <div style={{ textAlign: "center" }}>Type</div>
        <div style={{ textAlign: "right" }}>Amount</div>
        <div style={{ textAlign: "center" }}>Token</div>
        <div style={{ textAlign: "center" }}>Status</div>
        <div style={{ textAlign: "right" }}>Date</div>
      </div>

      {/* Rows */}
      {transactions.map((tx) => (
        <div
          key={tx.tx_hash}
          style={{
            display: "grid",
            gridTemplateColumns: "1fr 80px 100px 100px 80px 120px",
            gap: "0.5rem",
            padding: "1rem",
            borderBottom: "1px solid #eee",
            alignItems: "center",
            minHeight: "60px",
          }}
        >
          {/* Transaction Hash + To/From */}
          <div>
            <a
              href={tx.explorer_url}
              target="_blank"
              rel="noopener noreferrer"
              style={{
                fontFamily: "monospace",
                fontSize: "0.875rem",
                color: "#007bff",
                textDecoration: "none",
              }}
            >
              {truncateHash(tx.tx_hash)}
            </a>
            <div
              style={{
                fontSize: "0.75rem",
                color: "#666",
                marginTop: "0.25rem",
              }}
            >
              {tx.direction === "sent" ? "To" : "From"}: {truncateHash(tx.direction === "sent" ? tx.to : tx.from)}
            </div>
          </div>

          {/* Direction */}
          <div style={{ textAlign: "center" }}>
            <span
              style={{
                display: "inline-block",
                padding: "0.125rem 0.5rem",
                borderRadius: "4px",
                fontSize: "0.75rem",
                fontWeight: "bold",
                backgroundColor: tx.direction === "sent" ? "#fff3cd" : "#d4edda",
                color: tx.direction === "sent" ? "#856404" : "#155724",
              }}
            >
              {tx.direction === "sent" ? "↗ Sent" : "↙ Received"}
            </span>
          </div>

          {/* Amount */}
          <div style={{ textAlign: "right", fontFamily: "monospace" }}>
            {tx.direction === "sent" ? "-" : "+"}{tx.amount}
          </div>

          {/* Token */}
          <div style={{ textAlign: "center" }}>
            <span
              style={{
                display: "inline-block",
                padding: "0.125rem 0.5rem",
                borderRadius: "4px",
                fontSize: "0.75rem",
                backgroundColor: tx.token === "native" ? "#e7f3ff" : "#e8f5e9",
                color: tx.token === "native" ? "#0056b3" : "#2e7d32",
              }}
            >
              {tx.token === "native" ? "AVAX" : "USDC"}
            </span>
          </div>

          {/* Status */}
          <div style={{ textAlign: "center" }}>
            <StatusBadge status={tx.status as "pending" | "confirmed" | "failed"} />
          </div>

          {/* Date */}
          <div style={{ textAlign: "right", fontSize: "0.875rem", color: "#666" }}>
            {formatDate(tx.timestamp)}
          </div>
        </div>
      ))}

      {/* Refresh button */}
      <div
        style={{
          padding: "1rem",
          borderTop: "1px solid #ddd",
          backgroundColor: "#f8f9fa",
          textAlign: "center",
        }}
      >
        <button
          onClick={fetchTransactions}
          style={{
            padding: "0.5rem 1rem",
            backgroundColor: "#6c757d",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: "pointer",
          }}
        >
          Refresh
        </button>
      </div>
    </div>
  );
}
