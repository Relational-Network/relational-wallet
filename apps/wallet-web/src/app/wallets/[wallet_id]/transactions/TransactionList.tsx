// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useEffect, useCallback, useRef } from "react";
import { ExternalLink } from "lucide-react";
import type { TransactionSummary, TransactionListResponse } from "@/lib/api";

interface TransactionListProps {
  walletId: string;
  /** Increment to trigger a re-fetch from a parent component */
  refreshKey?: number;
  className?: string;
}

const REUR_FUJI_ADDRESS = "0x76568bed5acf1a5cd888773c8cae9ea2a9131a63";
const PAGE_SIZE = 50;
const FETCH_DEDUPE_WINDOW_MS = 1200;

function tokenLabel(token: string): string {
  if (token === "native") return "AVAX";
  const normalized = token.toLowerCase();
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

export function TransactionList({ walletId, refreshKey, className }: TransactionListProps) {
  const [pages, setPages] = useState<TransactionSummary[][]>([]);
  const [pageCursors, setPageCursors] = useState<(string | null)[]>([null]);
  const [pageIndex, setPageIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(true);
  const [isPageTransitioning, setIsPageTransitioning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fetchInFlightRef = useRef<Promise<void> | null>(null);
  const fetchInFlightKeyRef = useRef<string | null>(null);
  const lastSuccessfulFetchAtRef = useRef<Map<string, number>>(new Map());
  const pagesRef = useRef<TransactionSummary[][]>([]);
  useEffect(() => {
    pagesRef.current = pages;
  }, [pages]);

  const fetchPage = useCallback(async (
    page: number,
    cursor: string | null,
    options?: { force?: boolean; fullLoading?: boolean; activate?: boolean }
  ): Promise<boolean> => {
    const force = options?.force ?? false;
    const fullLoading = options?.fullLoading ?? false;
    const activate = options?.activate ?? false;
    const fetchKey = cursor ?? "__first__";

    if (
      fetchInFlightRef.current &&
      fetchInFlightKeyRef.current === fetchKey
    ) {
      await fetchInFlightRef.current;
      if (activate && pagesRef.current[page] !== undefined) {
        setPageIndex(page);
      }
      return pagesRef.current[page] !== undefined;
    }

    if (!force) {
      const lastFetchAt = lastSuccessfulFetchAtRef.current.get(fetchKey);
      if (
        lastFetchAt &&
        Date.now() - lastFetchAt < FETCH_DEDUPE_WINDOW_MS &&
        pagesRef.current[page] !== undefined
      ) {
        if (activate) {
          setPageIndex(page);
        }
        return true;
      }
    }

    if (fullLoading) {
      setIsLoading(true);
      setError(null);
    } else {
      setIsPageTransitioning(true);
    }

    let currentFetch: Promise<void> | null = null;
    currentFetch = (async () => {
      try {
        const params = new URLSearchParams({
          network: "fuji",
          limit: String(PAGE_SIZE),
        });
        if (cursor) {
          params.set("cursor", cursor);
        }

        const response = await fetch(
          `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/transactions?${params.toString()}`,
          {
            method: "GET",
            credentials: "include",
          }
        );

        if (response.ok) {
          const data: TransactionListResponse = await response.json();
          setPages((previous) => {
            const next = [...previous];
            next[page] = data.transactions;
            return next;
          });
          setPageCursors((previous) => {
            const next = [...previous];
            next[page] = cursor;
            next[page + 1] = data.next_cursor ?? null;
            if (data.next_cursor === null || data.next_cursor === undefined) {
              next.length = page + 2;
            }
            return next;
          });
          lastSuccessfulFetchAtRef.current.set(fetchKey, Date.now());
          if (activate) {
            setPageIndex(page);
          }
        } else if (response.status === 404) {
          setPages((previous) => {
            const next = [...previous];
            next[page] = [];
            return next;
          });
          setPageCursors((previous) => {
            const next = [...previous];
            next[page] = cursor;
            next[page + 1] = null;
            next.length = page + 2;
            return next;
          });
          lastSuccessfulFetchAtRef.current.set(fetchKey, Date.now());
          if (activate) {
            setPageIndex(page);
          }
        } else {
          const text = await response.text();
          setError(text || `Failed to load transactions (${response.status})`);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Network error");
      } finally {
        if (fullLoading) {
          setIsLoading(false);
        } else {
          setIsPageTransitioning(false);
        }
      }
    })().finally(() => {
      if (fetchInFlightRef.current === currentFetch) {
        fetchInFlightRef.current = null;
        fetchInFlightKeyRef.current = null;
      }
    });

    fetchInFlightRef.current = currentFetch;
    fetchInFlightKeyRef.current = fetchKey;
    await currentFetch;
    return pagesRef.current[page] !== undefined;
  }, [walletId]);

  const loadFirstPage = useCallback(async () => {
    setPages([]);
    pagesRef.current = [];
    setPageCursors([null]);
    setPageIndex(0);
    lastSuccessfulFetchAtRef.current.clear();
    setError(null);
    await fetchPage(0, null, { force: true, fullLoading: true, activate: true });
  }, [fetchPage]);

  useEffect(() => {
    void loadFirstPage();
  }, [loadFirstPage]);

  // Re-fetch when parent signals a change (e.g. after sending a transaction)
  const initialKeyRef = useRef(refreshKey);
  useEffect(() => {
    if (refreshKey === undefined || refreshKey === initialKeyRef.current) return;
    void loadFirstPage();
  }, [loadFirstPage, refreshKey]);

  const currentTransactions = pages[pageIndex] ?? [];
  const hasPreviousPage = pageIndex > 0;
  const nextPageCursor = pageCursors[pageIndex + 1];
  const hasCachedNextPage = pages[pageIndex + 1] !== undefined;
  const hasNextPage = hasCachedNextPage || (typeof nextPageCursor === "string" && nextPageCursor.length > 0);
  const pageSummary = `${currentTransactions.length} transaction${currentTransactions.length === 1 ? "" : "s"} on this page`;

  if (isLoading) {
    return (
      <article className={`card pad transaction-list-shell transaction-list-shell-loading${className ? ` ${className}` : ""}`}>
        <div className="transaction-list-meta">
          <div>
            <div className="skeleton" style={{ width: "18rem", maxWidth: "100%", height: "0.875rem" }} />
          </div>
          <div className="transaction-page-indicator" aria-hidden="true">
            Page 1
          </div>
        </div>
        <div className="transaction-table-shell transaction-table-shell-loading" aria-hidden="true">
          <div className="transaction-table-loading-header">
            <span>Transaction</span>
            <span>Direction</span>
            <span>Amount</span>
            <span>Token</span>
            <span>Status</span>
            <span>Date</span>
          </div>
          {[1, 2, 3, 4, 5].map((row) => (
            <div key={row} className="transaction-table-loading-row">
              <div className="transaction-table-loading-cell transaction-table-loading-cell-primary">
                <div className="skeleton" style={{ width: row % 2 === 0 ? "9.5rem" : "10.75rem", height: "0.875rem" }} />
                <div className="skeleton" style={{ width: "7rem", height: "0.625rem", marginTop: "0.35rem" }} />
              </div>
              <div className="skeleton" style={{ width: "4.5rem", height: "1.5rem", borderRadius: "999px" }} />
              <div className="skeleton" style={{ width: "4rem", height: "0.875rem" }} />
              <div className="skeleton" style={{ width: "3rem", height: "0.875rem" }} />
              <div className="skeleton" style={{ width: "5rem", height: "1.5rem", borderRadius: "999px" }} />
              <div className="skeleton" style={{ width: "6rem", height: "0.875rem" }} />
            </div>
          ))}
        </div>
      </article>
    );
  }

  if (error && currentTransactions.length === 0) {
    return (
      <article className="card pad">
        <div className="alert error">{error}</div>
        <div style={{ marginTop: "0.75rem" }}>
          <button onClick={() => void loadFirstPage()} className="btn btn-primary">
            Retry
          </button>
        </div>
      </article>
    );
  }

  if (currentTransactions.length === 0) {
    return (
      <article className="card pad">
        <div className="alert success">No transactions yet. Send or receive funds to populate history.</div>
      </article>
    );
  }

  return (
    <article className={`card pad transaction-list-shell${className ? ` ${className}` : ""}`}>
      <div className="transaction-list-meta">
        <div>
          <p className="helper-text" style={{ margin: 0 }}>
            Your full transaction history, with the newest activity first.
          </p>
        </div>
        <div className="transaction-page-indicator" aria-live="polite">
          Page {pageIndex + 1}
        </div>
      </div>

      <div className="transaction-table-shell">
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
            {currentTransactions.map((tx) => (
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
      {error ? (
        <div style={{ marginTop: "0.75rem" }} className="alert error">
          {error}
        </div>
      ) : null}
      {hasPreviousPage || hasNextPage ? (
        <div className="transaction-pagination">
          <div className="transaction-pagination-summary" aria-live="polite">
            <span className="transaction-page-indicator compact">Page {pageIndex + 1}</span>
            <span>{pageSummary}</span>
          </div>
          <div className="transaction-pagination-actions">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => setPageIndex((current) => Math.max(0, current - 1))}
              disabled={!hasPreviousPage || isPageTransitioning}
            >
              Previous page
            </button>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => {
                if (hasCachedNextPage) {
                  setPageIndex((current) => current + 1);
                  return;
                }
                if (typeof nextPageCursor === "string" && nextPageCursor.length > 0) {
                  void fetchPage(pageIndex + 1, nextPageCursor, {
                    activate: true,
                  });
                }
              }}
              disabled={!hasNextPage || isPageTransitioning}
            >
              {isPageTransitioning ? "Loading..." : "Next page"}
            </button>
          </div>
        </div>
      ) : null}
    </article>
  );
}
