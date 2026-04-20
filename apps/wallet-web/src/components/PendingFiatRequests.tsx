// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { ArrowDownLeft, ArrowUpRight, Clock, AlertCircle, AlertTriangle } from "lucide-react";
import type { FiatRequest, FiatRequestListResponse } from "@/lib/api";
import { ActionDialog } from "@/components/ActionDialog";

const REUR_FUJI_ADDRESS = "0x76568bed5acf1a5cd888773c8cae9ea2a9131a63";

const ACTIVE_STATUSES: FiatRequest["status"][] = [
  "queued",
  "awaiting_provider",
  "awaiting_user_deposit",
  "settlement_pending",
  "provider_pending",
];

interface BalanceResponse {
  token_balances: Array<{ symbol: string; balance_formatted: string }>;
}

interface PendingFiatRequestsProps {
  walletId: string;
  onProviderPopup?: (url: string) => void;
  onInvalidateWalletSnapshot?: () => void;
  refreshNonce?: number;
  /** Optimistically injected request — shown immediately before the next fetch. */
  latestRequest?: FiatRequest | null;
}

function statusLabel(status: FiatRequest["status"]): string {
  switch (status) {
    case "queued": return "Processing";
    case "awaiting_provider": return "Awaiting authorization";
    case "awaiting_user_deposit": return "Action required";
    case "settlement_pending": return "Processing";
    case "provider_pending": return "Processing";
    case "completed": return "Completed";
    case "failed": return "Failed";
    default: return status;
  }
}

function statusChipClass(status: FiatRequest["status"]): string {
  if (status === "awaiting_user_deposit") return "status-chip action";
  if (status === "completed") return "status-chip success";
  if (status === "failed") return "status-chip failed";
  return "status-chip warn";
}

function formatDate(value: string): string {
  return new Date(value).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function shortenAddress(address: string): string {
  if (address.length < 16) return address;
  return `${address.slice(0, 8)}...${address.slice(-6)}`;
}

/** Translate raw backend failure_reason into a user-friendly message. */
function humanizeFailureReason(reason: string): string {
  const lower = reason.toLowerCase();
  if (lower.includes("insufficient funds") || lower.includes("avax for gas")) {
    return "Settlement delayed — waiting for gas funding. Will retry automatically.";
  }
  if (lower.includes("insufficient balance") || lower.includes("not enough funds")) {
    return "Settlement delayed — waiting for gas funding. Will retry automatically.";
  }
  const attemptMatch = reason.match(/Attempt (\d+)\/(\d+)/);
  if (attemptMatch) {
    return `Settlement retry ${attemptMatch[1]}/${attemptMatch[2]} — will retry shortly.`;
  }
  return reason;
}

export function PendingFiatRequests({
  walletId,
  onProviderPopup,
  onInvalidateWalletSnapshot,
  refreshNonce,
  latestRequest,
}: PendingFiatRequestsProps) {
  const [requests, setRequests] = useState<FiatRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [depositRequest, setDepositRequest] = useState<FiatRequest | null>(null);
  const [reurBalance, setReurBalance] = useState<string | null>(null);
  const [loadingBalance, setLoadingBalance] = useState(false);
  const [sending, setSending] = useState(false);
  const [sendResult, setSendResult] = useState<{ success: boolean; message: string } | null>(null);

  // Stable refs for callbacks so fetchRequests doesn't depend on their identity
  const onInvalidateWalletSnapshotRef = useRef(onInvalidateWalletSnapshot);
  useEffect(() => {
    onInvalidateWalletSnapshotRef.current = onInvalidateWalletSnapshot;
  }, [onInvalidateWalletSnapshot]);

  // Track previous active request IDs so we can detect completions
  const prevActiveIdsRef = useRef<Set<string>>(new Set());
  const fetchInFlightRef = useRef<Promise<void> | null>(null);
  const lastFetchAtRef = useRef(0);
  // Track the initial refreshNonce so we skip fetch on first render
  const initialNonceRef = useRef(refreshNonce);
  // Burst-polling after deposit/action: poll every 3s for fast updates
  const [burstPolling, setBurstPolling] = useState(false);
  const burstTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const stopBurstPolling = useCallback(() => {
    if (burstTimerRef.current) {
      clearTimeout(burstTimerRef.current);
      burstTimerRef.current = null;
    }
    setBurstPolling(false);
  }, []);

  const startBurstPolling = useCallback(() => {
    setBurstPolling(true);
    if (burstTimerRef.current) {
      clearTimeout(burstTimerRef.current);
    }
    burstTimerRef.current = setTimeout(() => {
      setBurstPolling(false);
      burstTimerRef.current = null;
    }, 90_000);
  }, []);

  const fetchRequests = useCallback(async (force = false) => {
    const now = Date.now();
    if (!force) {
      if (fetchInFlightRef.current) {
        await fetchInFlightRef.current;
        return;
      }
      if (now - lastFetchAtRef.current < 2500) {
        return;
      }
    }

    let currentFetch: Promise<void> | null = null;
    currentFetch = (async () => {
      try {
        const params = new URLSearchParams({
          wallet_id: walletId,
          active_only: "true",
          limit: "25",
        });
        const response = await fetch(
          `/api/proxy/v1/fiat/requests?${params.toString()}`,
          { method: "GET", credentials: "include" }
        );
        if (!response.ok) return;

        const payload: FiatRequestListResponse = await response.json();
        const active = payload.requests.filter((r) =>
          ACTIVE_STATUSES.includes(r.status)
        );
        const mergedActive =
          latestRequest &&
          ACTIVE_STATUSES.includes(latestRequest.status)
            ? active.map((request) =>
                request.request_id === latestRequest.request_id &&
                request.status === "awaiting_user_deposit" &&
                latestRequest.status !== "awaiting_user_deposit"
                  ? latestRequest
                  : request
              )
            : active;

        // Detect requests that were previously active but are no longer
        const newActiveIds = new Set(mergedActive.map((r) => r.request_id));
        const prevIds = prevActiveIdsRef.current;
        if (prevIds.size > 0) {
          let anyCompleted = false;
          for (const id of prevIds) {
            if (!newActiveIds.has(id)) {
              anyCompleted = true;
              break;
            }
          }
          if (anyCompleted) {
            onInvalidateWalletSnapshotRef.current?.();
          }
        }

        prevActiveIdsRef.current = newActiveIds;
        setRequests(mergedActive);
        if (mergedActive.length === 0) {
          stopBurstPolling();
        }
      } catch {
        // Non-critical.
      } finally {
        setLoading(false);
      }
    })().finally(() => {
      if (fetchInFlightRef.current === currentFetch) {
        fetchInFlightRef.current = null;
      }
    });

    fetchInFlightRef.current = currentFetch;
    lastFetchAtRef.current = now;
    await currentFetch;
  }, [latestRequest, stopBurstPolling, walletId]);

  useEffect(() => {
    void fetchRequests(true);
  }, [fetchRequests]);

  useEffect(() => {
    if (refreshNonce === undefined) return;
    // Skip the initial render — the mount effect already fetches
    if (refreshNonce === initialNonceRef.current) return;
    void fetchRequests(true);
  }, [fetchRequests, refreshNonce]);

  // ── Optimistic insert: show latest request immediately ──────────
  useEffect(() => {
    if (!latestRequest) return;
    if (!ACTIVE_STATUSES.includes(latestRequest.status)) return;
    setRequests((prev) => {
      // Deduplicate: if already present, replace it
      const idx = prev.findIndex((r) => r.request_id === latestRequest.request_id);
      if (idx >= 0) {
        const next = [...prev];
        next[idx] = latestRequest;
        return next;
      }
      return [latestRequest, ...prev];
    });
    setLoading(false);
    // Also enable burst polling so we pick up status changes quickly
    startBurstPolling();
  }, [latestRequest, startBurstPolling]);

  // Poll to pick up status changes.
  // Burst mode (after deposit/action): 3s intervals for near-instant feedback.
  // Normal: 15s when provider/settlement is active, 60s otherwise.
  const hasSettling = requests.some(
    (r) => r.status === "settlement_pending" || r.status === "provider_pending"
  );
  useEffect(() => {
    if (requests.length === 0 && !loading && !burstPolling) return;
    const interval = setInterval(
      () => void fetchRequests(),
      burstPolling ? 3_000 : hasSettling ? 15_000 : 60_000
    );
    return () => clearInterval(interval);
  }, [fetchRequests, requests.length, loading, hasSettling, burstPolling]);

  useEffect(() => () => {
    if (burstTimerRef.current) {
      clearTimeout(burstTimerRef.current);
      burstTimerRef.current = null;
    }
  }, []);

  const fetchBalance = useCallback(async () => {
    setLoadingBalance(true);
    try {
      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/balance?network=fuji`,
        { method: "GET", credentials: "include" }
      );
      if (response.ok) {
        const data: BalanceResponse = await response.json();
        const reur = data.token_balances.find(
          (t) => t.symbol.toUpperCase() === "REUR"
        );
        setReurBalance(reur?.balance_formatted ?? "0");
      }
    } catch {
      // Keep null.
    } finally {
      setLoadingBalance(false);
    }
  }, [walletId]);

  const openDepositDialog = (request: FiatRequest) => {
    setDepositRequest(request);
    setSendResult(null);
    void fetchBalance();
  };

  const confirmDeposit = useCallback(async () => {
    if (!depositRequest?.service_wallet_address) return;

    setSending(true);
    setSendResult(null);

    try {
      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/send`,
        {
          method: "POST",
          credentials: "include",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            to: depositRequest.service_wallet_address,
            amount: depositRequest.amount_eur,
            token: REUR_FUJI_ADDRESS,
            network: "fuji",
          }),
        }
      );

      if (!response.ok) {
        const text = await response.text();
        setSendResult({ success: false, message: text || `Transfer failed (${response.status})` });
        return;
      }

      setSendResult({
        success: true,
        message: "Transfer submitted. The off-ramp will continue once the transaction confirms.",
      });
      // Optimistically update the request status so the UI shows
      // "Processing" immediately instead of staying on "Action required".
      setRequests((prev) =>
        prev.map((r) =>
          r.request_id === depositRequest.request_id
            ? { ...r, status: "provider_pending" as FiatRequest["status"] }
            : r
        )
      );
      setDepositRequest(null);
      onInvalidateWalletSnapshotRef.current?.();
      // Enable burst polling (3s) to pick up deposit detection + provider status quickly
      startBurstPolling();
      // Immediate refetch to pick up chain state
      void fetchRequests(true);
    } catch (err) {
      setSendResult({
        success: false,
        message: err instanceof Error ? err.message : "Network error",
      });
    } finally {
      setSending(false);
    }
  }, [depositRequest, fetchRequests, startBurstPolling, walletId]);

  if (loading && requests.length === 0) return null;
  if (requests.length === 0) return null;

  const parsedBalance = reurBalance !== null ? Number.parseFloat(reurBalance) : null;
  const depositAmount = depositRequest ? Number.parseFloat(depositRequest.amount_eur) : 0;
  const insufficient = parsedBalance !== null && parsedBalance < depositAmount;

  return (
    <>
      <div className="card card-pad">
        <div className="section-header">
          <h3 className="section-title">Pending Requests</h3>
          <button
            type="button"
            className="btn btn-ghost"
            onClick={() => void fetchRequests(true)}
            style={{ fontSize: "0.75rem" }}
          >
            Refresh
          </button>
        </div>

        <div style={{ marginTop: "0.5rem" }}>
          {requests.map((request) => (
            <div key={request.request_id} className="pending-request-card">
              <div className="pending-request-info">
                <div className="pending-request-title">
                  {request.direction === "on_ramp" ? (
                    <ArrowDownLeft size={14} style={{ color: "var(--success)" }} />
                  ) : (
                    <ArrowUpRight size={14} style={{ color: "var(--brand)" }} />
                  )}
                  <span>
                    {request.direction === "on_ramp" ? "On-Ramp" : "Off-Ramp"}{" "}
                    EUR {request.amount_eur}
                  </span>
                  <span className={statusChipClass(request.status)}>
                    {statusLabel(request.status)}
                  </span>
                </div>
                <div className="pending-request-meta">
                  <Clock size={11} style={{ verticalAlign: "-1px", marginRight: "0.25rem" }} />
                  {formatDate(request.created_at)}
                </div>
                {request.status === "settlement_pending" && request.failure_reason && (
                  <div
                    className="pending-request-meta"
                    style={{ color: "var(--warning-600, #b45309)", display: "flex", alignItems: "center", gap: "0.25rem", marginTop: "0.15rem" }}
                  >
                    <AlertTriangle size={11} />
                    <span>{humanizeFailureReason(request.failure_reason)}</span>
                  </div>
                )}
              </div>

              <div style={{ flexShrink: 0 }}>
                {request.status === "awaiting_user_deposit" &&
                  request.service_wallet_address && (
                    <button
                      type="button"
                      className="btn btn-primary"
                      style={{ fontSize: "0.8125rem" }}
                      onClick={() => openDepositDialog(request)}
                    >
                      Transfer rEUR
                    </button>
                  )}
                {request.status === "awaiting_provider" &&
                  request.provider_action_url &&
                  onProviderPopup && (
                    <button
                      type="button"
                      className="btn btn-secondary"
                      style={{ fontSize: "0.8125rem" }}
                      onClick={() => onProviderPopup(request.provider_action_url!)}
                    >
                      Authorize
                    </button>
                  )}
              </div>
            </div>
          ))}
        </div>
      </div>

      <ActionDialog
        open={depositRequest !== null}
        onClose={() => {
          if (!sending) setDepositRequest(null);
        }}
        title="Confirm rEUR Transfer"
      >
        {depositRequest && (
          <div className="stack">
            {sendResult?.success ? (
              <>
                <div className="alert alert-success">{sendResult.message}</div>
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => setDepositRequest(null)}
                >
                  Close
                </button>
              </>
            ) : (
              <>
                <p className="text-secondary" style={{ margin: 0 }}>
                  Transfer rEUR to the reserve wallet to continue your off-ramp.
                </p>

                <div className="confirm-summary">
                  <div className="confirm-row">
                    <span className="confirm-label">Amount</span>
                    <span className="confirm-value large">
                      {depositRequest.amount_eur} rEUR
                    </span>
                  </div>
                  <div className="confirm-row">
                    <span className="confirm-label">Your balance</span>
                    <span
                      className={`confirm-value${insufficient ? " insufficient" : ""}`}
                    >
                      {loadingBalance
                        ? "..."
                        : reurBalance !== null
                          ? `${reurBalance} rEUR`
                          : "unavailable"}
                    </span>
                  </div>
                  <div className="confirm-row">
                    <span className="confirm-label">To</span>
                    <span className="confirm-value mono">
                      {shortenAddress(depositRequest.service_wallet_address ?? "")}
                    </span>
                  </div>
                  <div className="confirm-row">
                    <span className="confirm-label">Network</span>
                    <span className="confirm-value">Fuji testnet</span>
                  </div>
                </div>

                {insufficient && (
                  <div className="alert alert-warning" style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                    <AlertCircle size={16} />
                    Insufficient rEUR balance. You need {depositRequest.amount_eur} rEUR but have {reurBalance}.
                  </div>
                )}

                {sendResult && !sendResult.success && (
                  <div className="alert alert-error">{sendResult.message}</div>
                )}

                <div
                  className="inline-actions"
                  style={{ justifyContent: "flex-end", marginTop: "0.25rem" }}
                >
                  <button
                    type="button"
                    className="btn btn-ghost"
                    onClick={() => setDepositRequest(null)}
                    disabled={sending}
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => void confirmDeposit()}
                    disabled={sending || insufficient}
                  >
                    {sending ? "Sending..." : `Transfer ${depositRequest.amount_eur} rEUR`}
                  </button>
                </div>
              </>
            )}
          </div>
        )}
      </ActionDialog>
    </>
  );
}
