// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import Link from "next/link";
import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  FiatRequest,
  FiatRequestListResponse,
  FiatRequestStatus,
  WalletListResponse,
  WalletResponse,
} from "@/lib/api";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";

interface UserMeResponse {
  user_id: string;
  role: string;
  session_id?: string;
}

interface FiatServiceWalletStatusResponse {
  wallet_id: string;
  public_address: string;
  bootstrapped: boolean;
  chain_network: string;
  reur_contract_address: string;
  avax_balance: string;
  reur_balance: string;
  reur_balance_raw: string;
}

interface ReserveTransactionResponse {
  tx_hash: string;
  explorer_url: string;
  amount_eur: string;
  amount_minor: string;
}

interface FiatSyncResponse {
  request: FiatRequest;
}

type RunbookAction =
  | "context"
  | "service_wallet"
  | "bootstrap_wallet"
  | "reserve_topup"
  | "reserve_transfer"
  | "onramp_create"
  | "offramp_create"
  | "requests_list"
  | "request_sync";

function requestStatusClass(status: FiatRequestStatus) {
  if (status === "completed") return "status-chip success";
  if (status === "failed") return "status-chip failed";
  if (
    status === "provider_pending" ||
    status === "awaiting_provider" ||
    status === "awaiting_user_deposit" ||
    status === "settlement_pending"
  ) {
    return "status-chip warn";
  }
  return "status-chip pending";
}

async function parseError(response: Response): Promise<string> {
  const text = await response.text();
  if (!text) return `Request failed (${response.status})`;
  try {
    const parsed = JSON.parse(text) as { error?: string; message?: string };
    return parsed.error || parsed.message || text;
  } catch {
    return text;
  }
}

async function requestJson<T>(url: string, init: RequestInit): Promise<T> {
  const response = await fetch(url, init);
  if (!response.ok) {
    throw new Error(await parseError(response));
  }

  const text = await response.text();
  if (!text) return {} as T;
  return JSON.parse(text) as T;
}

export function BootstrapConsole() {
  const [currentUser, setCurrentUser] = useState<UserMeResponse | null>(null);
  const [wallets, setWallets] = useState<WalletResponse[]>([]);
  const [selectedWalletId, setSelectedWalletId] = useState<string>("");
  const [serviceWallet, setServiceWallet] = useState<FiatServiceWalletStatusResponse | null>(null);
  const [requests, setRequests] = useState<FiatRequest[]>([]);

  const [topupAmount, setTopupAmount] = useState("1000000.00");
  const [transferTo, setTransferTo] = useState("");
  const [transferAmount, setTransferAmount] = useState("250.00");
  const [onRampAmount, setOnRampAmount] = useState("25.00");
  const [onRampNote, setOnRampNote] = useState("bootstrap smoke");
  const [offRampAmount, setOffRampAmount] = useState("10.00");
  const [offRampNote, setOffRampNote] = useState("bootstrap smoke");
  const [offRampName, setOffRampName] = useState("Relational Test User");
  const [offRampIban, setOffRampIban] = useState("GB79CLRB04066800102649");
  const [syncRequestId, setSyncRequestId] = useState("");

  const [busyAction, setBusyAction] = useState<RunbookAction | null>(null);
  const [pageError, setPageError] = useState<string | null>(null);
  const [resultMessage, setResultMessage] = useState<string | null>(null);

  const isAdmin = currentUser?.role === "admin";
  const selectedWallet = useMemo(
    () => wallets.find((wallet) => wallet.wallet_id === selectedWalletId) ?? null,
    [wallets, selectedWalletId]
  );

  const clearFeedback = () => {
    setPageError(null);
    setResultMessage(null);
  };

  const refreshServiceWallet = useCallback(async () => {
    if (!isAdmin) return;
    try {
      const payload = await requestJson<FiatServiceWalletStatusResponse>(
        "/api/proxy/v1/admin/fiat/service-wallet",
        { method: "GET", credentials: "include" }
      );
      setServiceWallet(payload);
    } catch (error) {
      setServiceWallet(null);
      setPageError(error instanceof Error ? error.message : "Failed to load service wallet status");
    }
  }, [isAdmin]);

  const refreshRequests = useCallback(
    async (walletId: string) => {
      if (!walletId) return;
      try {
        const payload = await requestJson<FiatRequestListResponse>(
          `/api/proxy/v1/fiat/requests?wallet_id=${encodeURIComponent(walletId)}`,
          { method: "GET", credentials: "include" }
        );
        setRequests(payload.requests);
      } catch (error) {
        setPageError(error instanceof Error ? error.message : "Failed to load fiat requests");
      }
    },
    []
  );

  const loadContext = useCallback(async () => {
    setBusyAction("context");
    clearFeedback();

    try {
      const [userPayload, walletsPayload] = await Promise.all([
        requestJson<UserMeResponse>("/api/proxy/v1/users/me", {
          method: "GET",
          credentials: "include",
        }),
        requestJson<WalletListResponse>("/api/proxy/v1/wallets", {
          method: "GET",
          credentials: "include",
        }),
      ]);

      setCurrentUser(userPayload);
      setWallets(walletsPayload.wallets);

      const nextWalletId = walletsPayload.wallets[0]?.wallet_id ?? "";
      setSelectedWalletId((current) => current || nextWalletId);
      setResultMessage("Context loaded.");
    } catch (error) {
      setPageError(error instanceof Error ? error.message : "Failed to load bootstrap context");
    } finally {
      setBusyAction(null);
    }
  }, []);

  useEffect(() => {
    void loadContext();
  }, [loadContext]);

  useEffect(() => {
    if (!selectedWalletId) return;
    void refreshRequests(selectedWalletId);
  }, [selectedWalletId, refreshRequests]);

  useEffect(() => {
    if (!isAdmin) return;
    void refreshServiceWallet();
  }, [isAdmin, refreshServiceWallet]);

  const runAdminAction = async <T,>(
    action: RunbookAction,
    callback: () => Promise<T>,
    successMessage: (payload: T) => string
  ) => {
    if (!isAdmin) {
      setPageError("Admin role is required for this action.");
      return;
    }
    setBusyAction(action);
    clearFeedback();
    try {
      const payload = await callback();
      setResultMessage(successMessage(payload));
    } catch (error) {
      setPageError(error instanceof Error ? error.message : "Operation failed");
    } finally {
      setBusyAction(null);
    }
  };

  const createOnRampRequest = async () => {
    if (!selectedWalletId) {
      setPageError("Select a wallet first.");
      return;
    }
    setBusyAction("onramp_create");
    clearFeedback();
    try {
      const payload = await requestJson<FiatRequest>("/api/proxy/v1/fiat/onramp/requests", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wallet_id: selectedWalletId,
          amount_eur: onRampAmount.trim(),
          provider: "truelayer_sandbox",
          note: onRampNote.trim() || undefined,
        }),
      });
      setResultMessage(`On-ramp request created: ${payload.request_id} (${payload.status})`);
      await refreshRequests(selectedWalletId);
    } catch (error) {
      setPageError(error instanceof Error ? error.message : "On-ramp request failed");
    } finally {
      setBusyAction(null);
    }
  };

  const createOffRampRequest = async () => {
    if (!selectedWalletId) {
      setPageError("Select a wallet first.");
      return;
    }
    setBusyAction("offramp_create");
    clearFeedback();
    try {
      const payload = await requestJson<FiatRequest>("/api/proxy/v1/fiat/offramp/requests", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wallet_id: selectedWalletId,
          amount_eur: offRampAmount.trim(),
          provider: "truelayer_sandbox",
          note: offRampNote.trim() || undefined,
          beneficiary_account_holder_name: offRampName.trim(),
          beneficiary_iban: offRampIban.trim(),
        }),
      });
      setResultMessage(
        `Off-ramp request created: ${payload.request_id} (${payload.status})${payload.service_wallet_address ? ` · deposit to ${payload.service_wallet_address}` : ""}`
      );
      await refreshRequests(selectedWalletId);
    } catch (error) {
      setPageError(error instanceof Error ? error.message : "Off-ramp request failed");
    } finally {
      setBusyAction(null);
    }
  };

  return (
    <SimpleWalletShell
      topBar={
        <div className="row-between" style={{ width: "100%" }}>
          <div className="row">
            <span className="badge badge-brand">Fuji bootstrap</span>
            <strong style={{ fontSize: "0.875rem" }}>Fiat Setup Console</strong>
          </div>
          <div className="row">
            <Link href="/wallets" className="btn btn-ghost">
              Back to Wallets
            </Link>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => void loadContext()}
              disabled={busyAction === "context"}
            >
              {busyAction === "context" ? "Refreshing..." : "Refresh Context"}
            </button>
          </div>
        </div>
      }
    >
      <section className="stack">
        <article className="card card-pad">
          <h2 className="section-title">1. Identity + Role Check</h2>
          {currentUser ? (
            <div className="stack-sm" style={{ marginTop: "0.75rem" }}>
              <div className="row">
                <span className="text-secondary">User ID</span>
                <span className="mono-sm">{currentUser.user_id}</span>
              </div>
              <div className="row">
                <span className="text-secondary">Role</span>
                <span className={`status-chip ${isAdmin ? "success" : "pending"}`}>
                  {currentUser.role}
                </span>
              </div>
              {!isAdmin ? (
                <div className="alert alert-warning">
                  Current role is not `admin`. Set Clerk public metadata to{" "}
                  <code>{"{\"role\":\"admin\"}"}</code> and
                  refresh session before running admin setup actions.
                </div>
              ) : null}
            </div>
          ) : (
            <p className="text-muted" style={{ marginTop: "0.75rem" }}>
              Loading identity context...
            </p>
          )}
        </article>

        <article className="card card-pad">
          <h2 className="section-title">2. Service Wallet + Reserve Operations (Admin)</h2>
          <div className="row" style={{ marginTop: "0.75rem", flexWrap: "wrap" }}>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() =>
                void runAdminAction(
                  "service_wallet",
                  () =>
                    requestJson<FiatServiceWalletStatusResponse>(
                      "/api/proxy/v1/admin/fiat/service-wallet",
                      { method: "GET", credentials: "include" }
                    ),
                  (payload) => `Service wallet loaded: ${payload.public_address}`
                )
              }
              disabled={busyAction !== null}
            >
              Load Service Wallet
            </button>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() =>
                void runAdminAction(
                  "bootstrap_wallet",
                  () =>
                    requestJson<FiatServiceWalletStatusResponse>(
                      "/api/proxy/v1/admin/fiat/service-wallet/bootstrap",
                      { method: "POST", credentials: "include" }
                    ),
                  (payload) => `Service wallet bootstrapped: ${payload.public_address}`
                ).then(async () => {
                  await refreshServiceWallet();
                })
              }
              disabled={busyAction !== null}
            >
              Bootstrap Service Wallet
            </button>
          </div>

          {serviceWallet ? (
            <div className="stack-sm" style={{ marginTop: "0.75rem" }}>
              <div className="row">
                <span className="text-secondary">Address</span>
                <span className="mono-sm">{serviceWallet.public_address}</span>
              </div>
              <div className="row">
                <span className="text-secondary">Network</span>
                <span className="badge badge-neutral">{serviceWallet.chain_network}</span>
              </div>
              <div className="row">
                <span className="text-secondary">AVAX</span>
                <span className="mono-sm">{serviceWallet.avax_balance}</span>
              </div>
              <div className="row">
                <span className="text-secondary">rEUR</span>
                <span className="mono-sm">
                  {serviceWallet.reur_balance} ({serviceWallet.reur_balance_raw} minor)
                </span>
              </div>
            </div>
          ) : null}

          <div className="grid-2" style={{ marginTop: "0.9rem" }}>
            <div className="field">
              <label htmlFor="topupAmount">Top-up amount (EUR)</label>
              <input
                id="topupAmount"
                value={topupAmount}
                onChange={(event) => setTopupAmount(event.target.value)}
                inputMode="decimal"
              />
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() =>
                  void runAdminAction(
                    "reserve_topup",
                    () =>
                      requestJson<ReserveTransactionResponse>("/api/proxy/v1/admin/fiat/reserve/topup", {
                        method: "POST",
                        credentials: "include",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({ amount_eur: topupAmount.trim() }),
                      }),
                    (payload) => `Top-up submitted: ${payload.tx_hash}`
                  ).then(async () => {
                    await refreshServiceWallet();
                  })
                }
                disabled={busyAction !== null || !topupAmount.trim()}
              >
                Mint Reserve Top-up
              </button>
            </div>

            <div className="stack-sm">
              <div className="field">
                <label htmlFor="transferTo">Transfer destination</label>
                <input
                  id="transferTo"
                  value={transferTo}
                  onChange={(event) => setTransferTo(event.target.value)}
                  placeholder="0x..."
                  style={{ fontFamily: "var(--font-mono)" }}
                />
              </div>
              <div className="field">
                <label htmlFor="transferAmount">Transfer amount (EUR)</label>
                <input
                  id="transferAmount"
                  value={transferAmount}
                  onChange={(event) => setTransferAmount(event.target.value)}
                  inputMode="decimal"
                />
              </div>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() =>
                  void runAdminAction(
                    "reserve_transfer",
                    () =>
                      requestJson<ReserveTransactionResponse>("/api/proxy/v1/admin/fiat/reserve/transfer", {
                        method: "POST",
                        credentials: "include",
                        headers: { "Content-Type": "application/json" },
                        body: JSON.stringify({
                          to: transferTo.trim(),
                          amount_eur: transferAmount.trim(),
                        }),
                      }),
                    (payload) => `Reserve transfer submitted: ${payload.tx_hash}`
                  ).then(async () => {
                    await refreshServiceWallet();
                  })
                }
                disabled={
                  busyAction !== null || !transferTo.trim() || !transferAmount.trim()
                }
              >
                Transfer rEUR From Reserve
              </button>
            </div>
          </div>
        </article>

        <article className="card card-pad">
          <h2 className="section-title">3. On/Off-Ramp Smoke Tests</h2>
          <div className="field" style={{ marginTop: "0.75rem" }}>
            <label htmlFor="walletSelect">Test wallet</label>
            <select
              id="walletSelect"
              value={selectedWalletId}
              onChange={(event) => setSelectedWalletId(event.target.value)}
            >
              {wallets.map((wallet) => (
                <option key={wallet.wallet_id} value={wallet.wallet_id}>
                  {(wallet.label || "Wallet")} · {wallet.wallet_id}
                </option>
              ))}
            </select>
          </div>
          {selectedWallet ? (
            <p className="mono-sm" style={{ marginTop: "0.5rem" }}>
              Wallet address: {selectedWallet.public_address}
            </p>
          ) : null}

          <div className="grid-2" style={{ marginTop: "0.9rem" }}>
            <div className="stack-sm">
              <h3 style={{ margin: 0, fontSize: "0.95rem" }}>On-ramp request</h3>
              <div className="field">
                <label htmlFor="onRampAmount">Amount (EUR)</label>
                <input
                  id="onRampAmount"
                  value={onRampAmount}
                  onChange={(event) => setOnRampAmount(event.target.value)}
                  inputMode="decimal"
                />
              </div>
              <div className="field">
                <label htmlFor="onRampNote">Note</label>
                <input
                  id="onRampNote"
                  value={onRampNote}
                  onChange={(event) => setOnRampNote(event.target.value)}
                />
              </div>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void createOnRampRequest()}
                disabled={busyAction !== null || !selectedWalletId || !onRampAmount.trim()}
              >
                {busyAction === "onramp_create" ? "Creating..." : "Create On-Ramp"}
              </button>
            </div>

            <div className="stack-sm">
              <h3 style={{ margin: 0, fontSize: "0.95rem" }}>Off-ramp request</h3>
              <div className="field">
                <label htmlFor="offRampAmount">Amount (EUR)</label>
                <input
                  id="offRampAmount"
                  value={offRampAmount}
                  onChange={(event) => setOffRampAmount(event.target.value)}
                  inputMode="decimal"
                />
              </div>
              <div className="field">
                <label htmlFor="offRampName">Beneficiary name</label>
                <input
                  id="offRampName"
                  value={offRampName}
                  onChange={(event) => setOffRampName(event.target.value)}
                />
              </div>
              <div className="field">
                <label htmlFor="offRampIban">Beneficiary IBAN</label>
                <input
                  id="offRampIban"
                  value={offRampIban}
                  onChange={(event) => setOffRampIban(event.target.value)}
                  style={{ fontFamily: "var(--font-mono)" }}
                />
              </div>
              <div className="field">
                <label htmlFor="offRampNote">Note</label>
                <input
                  id="offRampNote"
                  value={offRampNote}
                  onChange={(event) => setOffRampNote(event.target.value)}
                />
              </div>
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void createOffRampRequest()}
                disabled={
                  busyAction !== null ||
                  !selectedWalletId ||
                  !offRampAmount.trim() ||
                  !offRampName.trim() ||
                  !offRampIban.trim()
                }
              >
                {busyAction === "offramp_create" ? "Creating..." : "Create Off-Ramp"}
              </button>
            </div>
          </div>

          <div className="row" style={{ marginTop: "0.95rem", flexWrap: "wrap" }}>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => void refreshRequests(selectedWalletId)}
              disabled={busyAction !== null || !selectedWalletId}
            >
              Refresh Fiat Requests
            </button>
            <input
              value={syncRequestId}
              onChange={(event) => setSyncRequestId(event.target.value)}
              placeholder="request_id for admin sync"
              className="input"
              style={{ minWidth: "16rem" }}
            />
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() =>
                void runAdminAction(
                  "request_sync",
                  () =>
                    requestJson<FiatSyncResponse>(
                      `/api/proxy/v1/admin/fiat/requests/${encodeURIComponent(syncRequestId.trim())}/sync`,
                      { method: "POST", credentials: "include" }
                    ),
                  (payload) =>
                    `Request synced: ${payload.request.request_id} (${payload.request.status})`
                ).then(async () => {
                  await refreshRequests(selectedWalletId);
                })
              }
              disabled={busyAction !== null || !syncRequestId.trim()}
            >
              Admin Sync Request
            </button>
          </div>

          {requests.length > 0 ? (
            <div className="stack-sm" style={{ marginTop: "0.9rem" }}>
              {requests.slice(0, 10).map((request) => (
                <div key={request.request_id} className="card card-pad">
                  <div className="row-between" style={{ flexWrap: "wrap" }}>
                    <strong>
                      {request.direction === "on_ramp" ? "On-Ramp" : "Off-Ramp"} · EUR{" "}
                      {request.amount_eur}
                    </strong>
                    <span className={requestStatusClass(request.status)}>{request.status}</span>
                  </div>
                  <p className="mono-sm" style={{ marginTop: "0.4rem" }}>
                    request_id: {request.request_id}
                  </p>
                  {request.service_wallet_address ? (
                    <p className="mono-sm">service_wallet: {request.service_wallet_address}</p>
                  ) : null}
                  {request.reserve_transfer_tx_hash ? (
                    <p className="mono-sm">reserve_tx: {request.reserve_transfer_tx_hash}</p>
                  ) : null}
                  {request.deposit_tx_hash ? (
                    <p className="mono-sm">deposit_tx: {request.deposit_tx_hash}</p>
                  ) : null}
                  {request.failure_reason ? (
                    <p className="text-muted" style={{ color: "var(--danger)" }}>
                      failure: {request.failure_reason}
                    </p>
                  ) : null}
                </div>
              ))}
            </div>
          ) : (
            <p className="text-muted" style={{ marginTop: "0.75rem" }}>
              No fiat requests for this wallet yet.
            </p>
          )}
        </article>

        {resultMessage ? <div className="alert alert-success">{resultMessage}</div> : null}
        {pageError ? <div className="alert alert-error">{pageError}</div> : null}
      </section>
    </SimpleWalletShell>
  );
}
