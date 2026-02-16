// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { AlertCircle } from "lucide-react";
import type {
  FiatProviderListResponse,
  FiatProviderSummary,
  FiatRequest,
  FiatRequestListResponse,
} from "@/lib/api";
import { ActionDialog } from "@/components/ActionDialog";

interface FiatRequestPanelProps {
  walletId: string;
}

const DEFAULT_PROVIDER = "truelayer_sandbox";
const REUR_FUJI_ADDRESS = "0x76568bed5acf1a5cd888773c8cae9ea2a9131a63";

function formatDate(value: string): string {
  return new Date(value).toLocaleString();
}

function requestStatusClass(status: FiatRequest["status"]) {
  if (status === "completed") return "status-chip success";
  if (status === "failed") return "status-chip failed";
  if (status === "awaiting_user_deposit") return "status-chip action";
  if (
    status === "provider_pending" ||
    status === "awaiting_provider" ||
    status === "settlement_pending"
  ) {
    return "status-chip warn";
  }
  return "status-chip pending";
}

function shortenAddress(address: string): string {
  if (address.length < 16) return address;
  return `${address.slice(0, 8)}...${address.slice(-6)}`;
}

interface BalanceResponse {
  token_balances: Array<{ symbol: string; balance_formatted: string }>;
}

export function FiatRequestPanel({ walletId }: FiatRequestPanelProps) {
  const [providers, setProviders] = useState<FiatProviderSummary[]>([
    {
      provider_id: DEFAULT_PROVIDER,
      display_name: "TrueLayer Sandbox",
      sandbox: true,
      enabled: true,
      supports_on_ramp: true,
      supports_off_ramp: true,
    },
  ]);
  const [selectedProvider, setSelectedProvider] = useState(DEFAULT_PROVIDER);
  const [requests, setRequests] = useState<FiatRequest[]>([]);
  const [amountOnRamp, setAmountOnRamp] = useState("25");
  const [noteOnRamp, setNoteOnRamp] = useState("");
  const [amountOffRamp, setAmountOffRamp] = useState("10");
  const [noteOffRamp, setNoteOffRamp] = useState("");
  const [beneficiaryNameOffRamp, setBeneficiaryNameOffRamp] = useState("");
  const [beneficiaryIbanOffRamp, setBeneficiaryIbanOffRamp] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState<"on" | "off" | null>(null);
  const [isOnRampDialogOpen, setIsOnRampDialogOpen] = useState(false);
  const [isOffRampDialogOpen, setIsOffRampDialogOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Deposit confirmation state
  const [depositRequest, setDepositRequest] = useState<FiatRequest | null>(null);
  const [reurBalance, setReurBalance] = useState<string | null>(null);
  const [loadingBalance, setLoadingBalance] = useState(false);
  const [sending, setSending] = useState(false);
  const [sendResult, setSendResult] = useState<{ success: boolean; message: string } | null>(null);

  // Provider popup
  const providerPopupRef = useRef<Window | null>(null);

  const openProviderPopup = useCallback((url: string) => {
    const width = 500;
    const height = 700;
    const left = Math.round(window.screenX + (window.outerWidth - width) / 2);
    const top = Math.round(window.screenY + (window.outerHeight - height) / 2);
    const popup = window.open(
      url,
      "fiat-provider",
      `width=${width},height=${height},left=${left},top=${top},toolbar=no,menubar=no,location=yes`
    );
    if (popup) {
      providerPopupRef.current = popup;
    } else {
      window.open(url, "_blank", "noopener,noreferrer");
    }
  }, []);

  const fetchRequests = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await fetch(
        `/api/proxy/v1/fiat/requests?wallet_id=${encodeURIComponent(walletId)}`,
        {
          method: "GET",
          credentials: "include",
        }
      );

      if (!response.ok) {
        const text = await response.text();
        setError(text || `Failed to load fiat requests (${response.status})`);
        return;
      }

      const payload: FiatRequestListResponse = await response.json();
      setRequests(payload.requests);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    } finally {
      setIsLoading(false);
    }
  }, [walletId]);

  const fetchProviders = useCallback(async () => {
    try {
      const response = await fetch("/api/proxy/v1/fiat/providers", {
        method: "GET",
        credentials: "include",
      });

      if (!response.ok) {
        return;
      }

      const payload: FiatProviderListResponse = await response.json();
      if (!payload.providers.length) {
        return;
      }

      setProviders(payload.providers);
      setSelectedProvider((currentProvider) => {
        const hasSelected = payload.providers.some(
          (provider) => provider.provider_id === currentProvider
        );
        if (hasSelected) {
          return currentProvider;
        }
        return payload.default_provider || payload.providers[0].provider_id;
      });
    } catch {
      // Keep default provider fallback if provider discovery fails.
    }
  }, []);

  useEffect(() => {
    void fetchRequests();
  }, [fetchRequests]);

  useEffect(() => {
    void fetchProviders();
  }, [fetchProviders]);

  // Listen for postMessage from callback popup
  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      if (event.data?.type === "fiat-callback-complete") {
        providerPopupRef.current?.close();
        providerPopupRef.current = null;
        setSuccess("Bank authorization completed. Refreshing...");
        void fetchRequests();
      }
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [fetchRequests]);

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

  const confirmDeposit = async () => {
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
        message: "Transfer submitted. The off-ramp will continue once confirmed.",
      });
      setTimeout(() => void fetchRequests(), 3000);
    } catch (err) {
      setSendResult({
        success: false,
        message: err instanceof Error ? err.message : "Network error",
      });
    } finally {
      setSending(false);
    }
  };

  const createRequest = async (direction: "on" | "off") => {
    setError(null);
    setSuccess(null);

    const amount = direction === "on" ? amountOnRamp : amountOffRamp;
    const note = direction === "on" ? noteOnRamp : noteOffRamp;
    if (!amount.trim()) {
      setError("Amount is required.");
      return;
    }
    if (direction === "off") {
      if (!beneficiaryNameOffRamp.trim()) {
        setError("Beneficiary account holder name is required for off-ramp.");
        return;
      }
      if (!beneficiaryIbanOffRamp.trim()) {
        setError("Beneficiary IBAN is required for off-ramp.");
        return;
      }
    }

    setIsSubmitting(direction);
    try {
      const endpoint =
        direction === "on"
          ? "/api/proxy/v1/fiat/onramp/requests"
          : "/api/proxy/v1/fiat/offramp/requests";

      const response = await fetch(endpoint, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wallet_id: walletId,
          amount_eur: amount.trim(),
          provider: selectedProvider,
          note: note.trim() || undefined,
          beneficiary_account_holder_name:
            direction === "off" ? beneficiaryNameOffRamp.trim() : undefined,
          beneficiary_iban: direction === "off" ? beneficiaryIbanOffRamp.trim() : undefined,
        }),
      });

      if (!response.ok) {
        const text = await response.text();
        setError(text || `Request failed (${response.status})`);
        return;
      }

      const payload: FiatRequest = await response.json();

      if (direction === "on") {
        setSuccess(`On-ramp request created: ${payload.request_id}`);
        setNoteOnRamp("");
        setIsOnRampDialogOpen(false);
        // Auto-open provider popup if available
        if (payload.provider_action_url) {
          openProviderPopup(payload.provider_action_url);
        }
      } else {
        setSuccess(
          `Off-ramp request created: ${payload.request_id}. Use the "Transfer rEUR" button below to continue.`
        );
        setNoteOffRamp("");
        setBeneficiaryNameOffRamp("");
        setBeneficiaryIbanOffRamp("");
        setIsOffRampDialogOpen(false);
      }
      await fetchRequests();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    } finally {
      setIsSubmitting(null);
    }
  };

  const selectedProviderConfig = providers.find(
    (provider) => provider.provider_id === selectedProvider
  );
  const isProviderEnabled = selectedProviderConfig?.enabled ?? false;
  const canCreateOnRamp = isProviderEnabled && (selectedProviderConfig?.supports_on_ramp ?? false);
  const canCreateOffRamp = isProviderEnabled && (selectedProviderConfig?.supports_off_ramp ?? false);
  const onRampDisabledReason = !isProviderEnabled
    ? "Selected provider is not configured on backend."
    : !selectedProviderConfig?.supports_on_ramp
      ? "On-ramp is disabled for this provider."
      : null;
  const offRampDisabledReason = !isProviderEnabled
    ? "Selected provider is not configured on backend."
    : !selectedProviderConfig?.supports_off_ramp
      ? "Off-ramp is disabled for this provider."
      : null;

  const parsedBalance = reurBalance !== null ? Number.parseFloat(reurBalance) : null;
  const depositAmount = depositRequest ? Number.parseFloat(depositRequest.amount_eur) : 0;
  const insufficient = parsedBalance !== null && parsedBalance < depositAmount;

  return (
    <section className="page-row">
      <article className="card pad">
        <h2 className="card-title">Provider</h2>
        <p className="card-subtitle">
          Fuji-only reserve flow. On-ramp releases rEUR after provider success; off-ramp pays out
          after your rEUR deposit is detected.
        </p>

        <div className="field" style={{ marginTop: "0.8rem" }}>
          <label htmlFor="fiatProvider">Provider</label>
          <select
            id="fiatProvider"
            value={selectedProvider}
            onChange={(event) => setSelectedProvider(event.target.value)}
          >
            {providers.map((provider) => (
              <option key={provider.provider_id} value={provider.provider_id}>
                {provider.display_name} ({provider.provider_id})
              </option>
            ))}
          </select>
        </div>

        {!isProviderEnabled && (
          <div className="alert warn" style={{ marginTop: "0.7rem" }}>
            Selected provider is not configured. Set required `TRUELAYER_*` env vars on backend.
          </div>
        )}
      </article>

      <div className="page-grid-2">
        <article className="card pad">
          <h3 className="card-title">On-ramp: Fiat to tokens</h3>
          <p className="card-subtitle">Use this when you want to buy tokens with fiat.</p>
          {onRampDisabledReason && (
            <p className="helper-text" style={{ color: "var(--bad-600)", marginTop: "0.6rem" }}>
              {onRampDisabledReason}
            </p>
          )}
          <div style={{ marginTop: "0.75rem" }}>
            <button
              type="button"
              onClick={() => {
                setError(null);
                setSuccess(null);
                setIsOnRampDialogOpen(true);
              }}
              title={onRampDisabledReason ?? undefined}
              disabled={isSubmitting !== null || !canCreateOnRamp}
              className="btn btn-primary"
            >
              New On-Ramp
            </button>
          </div>
        </article>

        <article className="card pad">
          <h3 className="card-title">Off-ramp: Tokens to fiat</h3>
          <p className="card-subtitle">Use this when you want to redeem tokens into fiat.</p>
          {offRampDisabledReason && (
            <p className="helper-text" style={{ color: "var(--bad-600)", marginTop: "0.6rem" }}>
              {offRampDisabledReason}
            </p>
          )}
          <div style={{ marginTop: "0.75rem" }}>
            <button
              type="button"
              onClick={() => {
                setError(null);
                setSuccess(null);
                setIsOffRampDialogOpen(true);
              }}
              title={offRampDisabledReason ?? undefined}
              disabled={isSubmitting !== null || !canCreateOffRamp}
              className="btn btn-soft"
            >
              New Off-Ramp
            </button>
          </div>
        </article>
      </div>

      <div className="inline-actions">
        <button type="button" onClick={() => void fetchRequests()} className="btn btn-ghost">
          Refresh
        </button>
      </div>

      {error && <div className="alert error">{error}</div>}
      {success && <div className="alert success">{success}</div>}

      <article className="card pad">
        <h3 className="card-title">Fiat requests</h3>
        {isLoading ? (
          <div className="list-stack" style={{ marginTop: "0.65rem" }}>
            <div className="skeleton-line" />
            <div className="skeleton-line" />
          </div>
        ) : requests.length === 0 ? (
          <p className="helper-text" style={{ marginTop: "0.6rem" }}>No fiat requests yet.</p>
        ) : (
          <div className="list-stack" style={{ marginTop: "0.7rem" }}>
            {requests.map((request) => (
              <div key={request.request_id} className="card pad" style={{ boxShadow: "none" }}>
                <div style={{ display: "flex", justifyContent: "space-between", gap: "0.5rem", flexWrap: "wrap" }}>
                  <div>
                    <strong>{request.direction === "on_ramp" ? "On-Ramp" : "Off-Ramp"} • EUR {request.amount_eur}</strong>
                    <div className="mono">{request.request_id}</div>
                  </div>
                  <span className={requestStatusClass(request.status)}>{request.status}</span>
                </div>
                <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.4rem" }}>
                  {request.provider} • {formatDate(request.created_at)}
                </p>
                <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.2rem" }}>
                  Network: {request.chain_network.toUpperCase()}
                </p>
                {request.service_wallet_address ? (
                  <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.2rem" }}>
                    Reserve wallet: <span className="mono">{request.service_wallet_address}</span>
                  </p>
                ) : null}
                {request.deposit_tx_hash ? (
                  <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.2rem" }}>
                    Deposit tx: <span className="mono">{request.deposit_tx_hash}</span>
                  </p>
                ) : null}
                {request.reserve_transfer_tx_hash ? (
                  <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.2rem" }}>
                    Settlement tx: <span className="mono">{request.reserve_transfer_tx_hash}</span>
                  </p>
                ) : null}
                {request.failure_reason ? (
                  <p
                    className="helper-text"
                    style={{ marginBottom: 0, marginTop: "0.2rem", color: "var(--bad-600)" }}
                  >
                    Failure: {request.failure_reason}
                  </p>
                ) : null}
                {request.note ? (
                  <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.2rem" }}>
                    Note: {request.note}
                  </p>
                ) : null}

                {/* Action buttons for pending requests */}
                {request.status === "awaiting_user_deposit" && request.service_wallet_address ? (
                  <div style={{ marginTop: "0.5rem" }}>
                    <button
                      type="button"
                      className="btn btn-primary"
                      style={{ fontSize: "0.8125rem" }}
                      onClick={() => openDepositDialog(request)}
                    >
                      Transfer {request.amount_eur} rEUR
                    </button>
                  </div>
                ) : null}
                {request.status === "awaiting_provider" && request.provider_action_url ? (
                  <div style={{ marginTop: "0.5rem" }}>
                    <button
                      type="button"
                      className="btn btn-secondary"
                      style={{ fontSize: "0.8125rem" }}
                      onClick={() => openProviderPopup(request.provider_action_url!)}
                    >
                      Authorize with bank
                    </button>
                  </div>
                ) : null}
              </div>
            ))}
          </div>
        )}
      </article>

      {isOnRampDialogOpen ? (
        <div className="dialog-backdrop" role="dialog" aria-modal="true" aria-label="On-ramp request">
          <div className="dialog-card">
            <h3>On-ramp request</h3>
            <p className="helper-text" style={{ marginTop: "0.45rem" }}>
              Create a sandbox bank payment authorization to mint token value.
            </p>
            <div className="field" style={{ marginTop: "0.75rem" }}>
              <label htmlFor="onRampAmount">Amount (EUR)</label>
              <input
                id="onRampAmount"
                type="text"
                value={amountOnRamp}
                onChange={(event) => setAmountOnRamp(event.target.value)}
                placeholder="25"
              />
            </div>
            <div className="field" style={{ marginTop: "0.65rem" }}>
              <label htmlFor="onRampNote">Note (optional)</label>
              <textarea
                id="onRampNote"
                value={noteOnRamp}
                onChange={(event) => setNoteOnRamp(event.target.value)}
                rows={2}
                placeholder="sandbox on-ramp"
              />
            </div>
            <div className="inline-actions" style={{ justifyContent: "flex-end", marginTop: "0.8rem" }}>
              <button type="button" className="btn btn-ghost" onClick={() => setIsOnRampDialogOpen(false)} disabled={isSubmitting !== null}>
                Cancel
              </button>
              <button type="button" className="btn btn-primary" onClick={() => void createRequest("on")} disabled={isSubmitting !== null}>
                {isSubmitting === "on" ? "Creating..." : "Create On-Ramp"}
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {isOffRampDialogOpen ? (
        <div className="dialog-backdrop" role="dialog" aria-modal="true" aria-label="Off-ramp request">
          <div className="dialog-card">
            <h3>Off-ramp request</h3>
            <p className="helper-text" style={{ marginTop: "0.45rem" }}>
              Provide beneficiary details for sandbox payout settlement.
            </p>
            <div className="field" style={{ marginTop: "0.75rem" }}>
              <label htmlFor="offRampAmount">Amount (EUR)</label>
              <input
                id="offRampAmount"
                type="text"
                value={amountOffRamp}
                onChange={(event) => setAmountOffRamp(event.target.value)}
                placeholder="10"
              />
            </div>
            <div className="field" style={{ marginTop: "0.65rem" }}>
              <label htmlFor="beneficiaryName">Beneficiary account holder name</label>
              <input
                id="beneficiaryName"
                type="text"
                value={beneficiaryNameOffRamp}
                onChange={(event) => setBeneficiaryNameOffRamp(event.target.value)}
                placeholder="Relational Bank 1"
              />
            </div>
            <div className="field" style={{ marginTop: "0.65rem" }}>
              <label htmlFor="beneficiaryIban">Beneficiary IBAN</label>
              <input
                id="beneficiaryIban"
                type="text"
                value={beneficiaryIbanOffRamp}
                onChange={(event) => setBeneficiaryIbanOffRamp(event.target.value)}
                placeholder="GB79CLRB04066800102649"
              />
            </div>
            <div className="field" style={{ marginTop: "0.65rem" }}>
              <label htmlFor="offRampNote">Note (optional)</label>
              <textarea
                id="offRampNote"
                value={noteOffRamp}
                onChange={(event) => setNoteOffRamp(event.target.value)}
                rows={2}
                placeholder="sandbox off-ramp"
              />
            </div>
            <div className="inline-actions" style={{ justifyContent: "flex-end", marginTop: "0.8rem" }}>
              <button type="button" className="btn btn-ghost" onClick={() => setIsOffRampDialogOpen(false)} disabled={isSubmitting !== null}>
                Cancel
              </button>
              <button type="button" className="btn btn-primary" onClick={() => void createRequest("off")} disabled={isSubmitting !== null}>
                {isSubmitting === "off" ? "Creating..." : "Create Off-Ramp"}
              </button>
            </div>
          </div>
        </div>
      ) : null}

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
    </section>
  );
}
