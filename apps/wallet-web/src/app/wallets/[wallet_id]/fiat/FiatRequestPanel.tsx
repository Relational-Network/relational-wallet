// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  FiatProviderListResponse,
  FiatProviderSummary,
  FiatRequest,
  FiatRequestListResponse,
} from "@/lib/api";

interface FiatRequestPanelProps {
  walletId: string;
}

const DEFAULT_PROVIDER = "truelayer_sandbox";

function formatDate(value: string): string {
  return new Date(value).toLocaleString();
}

function requestStatusClass(status: FiatRequest["status"]) {
  if (status === "completed") return "status-chip success";
  if (status === "failed") return "status-chip failed";
  if (status === "provider_pending") return "status-chip warn";
  return "status-chip pending";
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
  const [providerActionUrl, setProviderActionUrl] = useState<string | null>(null);

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

  const createRequest = async (direction: "on" | "off") => {
    setError(null);
    setSuccess(null);
    setProviderActionUrl(null);

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
      setSuccess(
        `${direction === "on" ? "On-ramp" : "Off-ramp"} request created: ${payload.request_id}`
      );
      setProviderActionUrl(payload.provider_action_url || null);
      if (direction === "on") {
        setNoteOnRamp("");
        setIsOnRampDialogOpen(false);
      } else {
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

  return (
    <section className="page-row">
      <article className="card pad">
        <h2 className="card-title">Provider</h2>
        <p className="card-subtitle">
          On-ramp converts fiat to tokens. Off-ramp converts tokens back to fiat.
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
                setProviderActionUrl(null);
                setIsOnRampDialogOpen(true);
              }}
              title={onRampDisabledReason ?? undefined}
              disabled={isSubmitting !== null || !canCreateOnRamp}
              className="btn btn-primary"
            >
              Open On-Ramp Dialog
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
                setProviderActionUrl(null);
                setIsOffRampDialogOpen(true);
              }}
              title={offRampDisabledReason ?? undefined}
              disabled={isSubmitting !== null || !canCreateOffRamp}
              className="btn btn-soft"
            >
              Open Off-Ramp Dialog
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

      {providerActionUrl && (
        <article className="card pad">
          <h3 className="card-title">Continue with bank authorization</h3>
          <p className="card-subtitle">Open provider session in a new tab and complete consent.</p>
          <div className="inline-actions" style={{ marginTop: "0.75rem" }}>
            <a href={providerActionUrl} target="_blank" rel="noopener noreferrer" className="btn btn-primary">
              Continue in provider
            </a>
          </div>
          <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.65rem" }}>
            Note: If the URL has expired in sandbox, create a fresh request.
          </p>
        </article>
      )}

      <article className="card pad">
        <h3 className="card-title">Recent fiat requests</h3>
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
                {request.note ? (
                  <p className="helper-text" style={{ marginBottom: 0, marginTop: "0.2rem" }}>
                    Note: {request.note}
                  </p>
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
    </section>
  );
}
