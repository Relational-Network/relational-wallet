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

function statusColor(status: FiatRequest["status"]): { bg: string; color: string } {
  switch (status) {
    case "completed":
      return { bg: "#def5e8", color: "#19663a" };
    case "failed":
      return { bg: "#fce2e2", color: "#a13030" };
    case "provider_pending":
      return { bg: "#fff3cd", color: "#8a5b00" };
    default:
      return { bg: "#e8f1fb", color: "#1f4f77" };
  }
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
    fetchRequests();
  }, [fetchRequests]);

  useEffect(() => {
    fetchProviders();
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

  return (
    <section
      style={{
        border: "1px solid #ddd",
        borderRadius: "8px",
        padding: "1rem",
        backgroundColor: "#fff",
      }}
    >
      <h2 style={{ marginTop: 0 }}>Fiat On-Ramp / Off-Ramp</h2>
      <p style={{ marginTop: 0, color: "#666", fontSize: "0.875rem" }}>
        On-ramp converts fiat to tokens. Off-ramp converts tokens back to fiat. Both flows use
        live sandbox provider APIs so production cutover remains configuration-driven.
      </p>
      {!isProviderEnabled && (
        <p style={{ marginTop: 0, color: "#b32424", fontSize: "0.875rem" }}>
          Selected provider is not configured on the backend yet. Set `TRUELAYER_*` env vars to
          enable live sandbox requests.
        </p>
      )}

      <div style={{ display: "grid", gap: "0.75rem", marginBottom: "1rem" }}>
        <div>
          <label htmlFor="fiatProvider" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            Provider
          </label>
          <select
            id="fiatProvider"
            value={selectedProvider}
            onChange={(event) => setSelectedProvider(event.target.value)}
            style={{
              width: "100%",
              boxSizing: "border-box",
              padding: "0.6rem",
              borderRadius: "4px",
              border: "1px solid #ddd",
              backgroundColor: "#fff",
            }}
          >
            {providers.map((provider) => (
              <option key={provider.provider_id} value={provider.provider_id}>
                {provider.display_name} ({provider.provider_id})
              </option>
            ))}
          </select>
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(240px, 1fr))",
          gap: "0.75rem",
          marginBottom: "1rem",
        }}
      >
        <div
          style={{
            border: "1px solid #d9e9f8",
            borderRadius: "8px",
            padding: "0.75rem",
            backgroundColor: "#f4f9ff",
          }}
        >
          <h3 style={{ margin: "0 0 0.4rem 0", fontSize: "1rem" }}>On-Ramp: Fiat to Tokens</h3>
          <p style={{ margin: "0 0 0.65rem 0", fontSize: "0.83rem", color: "#4b5966" }}>
            Use this when you want to buy tokens with fiat.
          </p>
          <button
            type="button"
            onClick={() => {
              setError(null);
              setSuccess(null);
              setProviderActionUrl(null);
              setIsOnRampDialogOpen(true);
            }}
            disabled={isSubmitting !== null || !canCreateOnRamp}
            style={{
              width: "100%",
              padding: "0.6rem 0.95rem",
              border: "none",
              borderRadius: "4px",
              backgroundColor: isSubmitting !== null || !canCreateOnRamp ? "#84b6dd" : "#0b7bd3",
              color: "#fff",
              cursor: isSubmitting !== null || !canCreateOnRamp ? "not-allowed" : "pointer",
              fontWeight: "bold",
            }}
          >
            Open On-Ramp Dialog
          </button>
        </div>

        <div
          style={{
            border: "1px solid #dbe3ea",
            borderRadius: "8px",
            padding: "0.75rem",
            backgroundColor: "#f7f9fc",
          }}
        >
          <h3 style={{ margin: "0 0 0.4rem 0", fontSize: "1rem" }}>Off-Ramp: Tokens to Fiat</h3>
          <p style={{ margin: "0 0 0.65rem 0", fontSize: "0.83rem", color: "#4b5966" }}>
            Use this when you want to redeem tokens into fiat.
          </p>
          <button
            type="button"
            onClick={() => {
              setError(null);
              setSuccess(null);
              setProviderActionUrl(null);
              setIsOffRampDialogOpen(true);
            }}
            disabled={isSubmitting !== null || !canCreateOffRamp}
            style={{
              width: "100%",
              padding: "0.6rem 0.95rem",
              border: "none",
              borderRadius: "4px",
              backgroundColor: isSubmitting !== null || !canCreateOffRamp ? "#8fa7be" : "#4f6f8b",
              color: "#fff",
              cursor: isSubmitting !== null || !canCreateOffRamp ? "not-allowed" : "pointer",
              fontWeight: "bold",
            }}
          >
            Open Off-Ramp Dialog
          </button>
        </div>
      </div>

      <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", marginBottom: "1rem" }}>
        <button
          type="button"
          onClick={fetchRequests}
          style={{
            padding: "0.6rem 0.95rem",
            border: "1px solid #adb5bd",
            borderRadius: "4px",
            backgroundColor: "#fff",
            color: "#444",
            cursor: "pointer",
          }}
        >
          Refresh
        </button>
      </div>

      {error && (
        <p style={{ margin: "0 0 0.75rem 0", color: "#b32424", fontSize: "0.875rem" }}>{error}</p>
      )}
      {success && (
        <p style={{ margin: "0 0 0.75rem 0", color: "#176c39", fontSize: "0.875rem" }}>{success}</p>
      )}
      {providerActionUrl && (
        <p style={{ margin: "0 0 0.75rem 0", fontSize: "0.875rem" }}>
          <a
            href={providerActionUrl}
            target="_blank"
            rel="noreferrer"
            style={{ color: "#0b7bd3", textDecoration: "underline" }}
          >
            Continue with bank authorization
          </a>
        </p>
      )}

      <h3 style={{ marginBottom: "0.6rem" }}>Recent Fiat Requests</h3>
      {isLoading ? (
        <p style={{ color: "#666", margin: 0 }}>Loading requests...</p>
      ) : requests.length === 0 ? (
        <p style={{ color: "#666", margin: 0 }}>No fiat requests yet.</p>
      ) : (
        <div style={{ display: "grid", gap: "0.55rem" }}>
          {requests.map((request) => {
            const colors = statusColor(request.status);
            return (
              <div
                key={request.request_id}
                style={{
                  border: "1px solid #e1e6eb",
                  borderRadius: "6px",
                  padding: "0.65rem",
                  backgroundColor: "#fafbfd",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem" }}>
                  <strong style={{ fontSize: "0.88rem" }}>
                    {request.direction === "on_ramp" ? "On-Ramp" : "Off-Ramp"} • EUR {request.amount_eur}
                  </strong>
                  <span
                    style={{
                      fontSize: "0.7rem",
                      padding: "0.18rem 0.45rem",
                      borderRadius: "999px",
                      backgroundColor: colors.bg,
                      color: colors.color,
                      fontWeight: "bold",
                    }}
                  >
                    {request.status}
                  </span>
                </div>
                <div style={{ marginTop: "0.35rem", color: "#556372", fontSize: "0.78rem" }}>
                  ID: <span style={{ fontFamily: "monospace" }}>{request.request_id}</span>
                </div>
                <div style={{ marginTop: "0.2rem", color: "#556372", fontSize: "0.78rem" }}>
                  Provider: {request.provider} • {formatDate(request.created_at)}
                </div>
                {request.provider_action_url && request.status !== "completed" && (
                  <div style={{ marginTop: "0.2rem", fontSize: "0.78rem" }}>
                    <a
                      href={request.provider_action_url}
                      target="_blank"
                      rel="noreferrer"
                      style={{ color: "#0b7bd3", textDecoration: "underline" }}
                    >
                      Continue in provider
                    </a>
                  </div>
                )}
                {request.note && (
                  <div style={{ marginTop: "0.2rem", color: "#556372", fontSize: "0.78rem" }}>
                    Note: {request.note}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {isOnRampDialogOpen && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="onRampDialogTitle"
          onClick={() => setIsOnRampDialogOpen(false)}
          style={{
            position: "fixed",
            inset: 0,
            backgroundColor: "rgba(10, 25, 41, 0.55)",
            display: "grid",
            placeItems: "center",
            zIndex: 60,
            padding: "1rem",
          }}
        >
          <div
            onClick={(event) => event.stopPropagation()}
            style={{
              width: "100%",
              maxWidth: "460px",
              backgroundColor: "#fff",
              borderRadius: "10px",
              border: "1px solid #d6e6f5",
              boxShadow: "0 20px 48px rgba(7, 24, 39, 0.2)",
              padding: "1rem",
            }}
          >
            <h3 id="onRampDialogTitle" style={{ marginTop: 0, marginBottom: "0.5rem" }}>
              Create On-Ramp Request
            </h3>
            <p style={{ marginTop: 0, marginBottom: "0.75rem", color: "#5a6773", fontSize: "0.85rem" }}>
              Fiat to tokens. Submit and continue in provider to authorize bank transfer.
            </p>
            <label
              htmlFor="onRampAmountDialog"
              style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}
            >
              Amount (EUR)
            </label>
            <input
              id="onRampAmountDialog"
              type="text"
              value={amountOnRamp}
              onChange={(event) => setAmountOnRamp(event.target.value)}
              style={{
                width: "100%",
                boxSizing: "border-box",
                padding: "0.6rem",
                borderRadius: "4px",
                border: "1px solid #ddd",
                marginBottom: "0.6rem",
              }}
            />
            <label
              htmlFor="onRampNoteDialog"
              style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}
            >
              Note (optional)
            </label>
            <input
              id="onRampNoteDialog"
              type="text"
              value={noteOnRamp}
              onChange={(event) => setNoteOnRamp(event.target.value)}
              maxLength={140}
              style={{
                width: "100%",
                boxSizing: "border-box",
                padding: "0.6rem",
                borderRadius: "4px",
                border: "1px solid #ddd",
              }}
            />
            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.6rem", marginTop: "0.9rem" }}>
              <button
                type="button"
                onClick={() => setIsOnRampDialogOpen(false)}
                style={{
                  padding: "0.55rem 0.9rem",
                  border: "1px solid #b9c7d5",
                  borderRadius: "4px",
                  backgroundColor: "#fff",
                  color: "#4c5a67",
                }}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => createRequest("on")}
                disabled={isSubmitting !== null || !canCreateOnRamp}
                style={{
                  padding: "0.55rem 0.9rem",
                  border: "none",
                  borderRadius: "4px",
                  backgroundColor:
                    isSubmitting !== null || !canCreateOnRamp ? "#84b6dd" : "#0b7bd3",
                  color: "#fff",
                  fontWeight: "bold",
                  cursor: isSubmitting !== null || !canCreateOnRamp ? "not-allowed" : "pointer",
                }}
              >
                {isSubmitting === "on" ? "Creating..." : "Create On-Ramp Request"}
              </button>
            </div>
          </div>
        </div>
      )}

      {isOffRampDialogOpen && (
        <div
          role="dialog"
          aria-modal="true"
          aria-labelledby="offRampDialogTitle"
          onClick={() => setIsOffRampDialogOpen(false)}
          style={{
            position: "fixed",
            inset: 0,
            backgroundColor: "rgba(14, 19, 34, 0.55)",
            display: "grid",
            placeItems: "center",
            zIndex: 60,
            padding: "1rem",
          }}
        >
          <div
            onClick={(event) => event.stopPropagation()}
            style={{
              width: "100%",
              maxWidth: "460px",
              backgroundColor: "#fff",
              borderRadius: "10px",
              border: "1px solid #d9e2eb",
              boxShadow: "0 20px 48px rgba(7, 24, 39, 0.2)",
              padding: "1rem",
            }}
          >
            <h3 id="offRampDialogTitle" style={{ marginTop: 0, marginBottom: "0.5rem" }}>
              Create Off-Ramp Request
            </h3>
            <p style={{ marginTop: 0, marginBottom: "0.75rem", color: "#5a6773", fontSize: "0.85rem" }}>
              Tokens to fiat. Submit and track payout status in request history.
            </p>
            <label
              htmlFor="offRampAmountDialog"
              style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}
            >
              Amount (EUR)
            </label>
            <input
              id="offRampAmountDialog"
              type="text"
              value={amountOffRamp}
              onChange={(event) => setAmountOffRamp(event.target.value)}
              style={{
                width: "100%",
                boxSizing: "border-box",
                padding: "0.6rem",
                borderRadius: "4px",
                border: "1px solid #ddd",
                marginBottom: "0.6rem",
              }}
            />
            <label
              htmlFor="offRampNoteDialog"
              style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}
            >
              Note (optional)
            </label>
            <input
              id="offRampNoteDialog"
              type="text"
              value={noteOffRamp}
              onChange={(event) => setNoteOffRamp(event.target.value)}
              maxLength={140}
              style={{
                width: "100%",
                boxSizing: "border-box",
                padding: "0.6rem",
                borderRadius: "4px",
                border: "1px solid #ddd",
              }}
            />
            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.6rem", marginTop: "0.9rem" }}>
              <button
                type="button"
                onClick={() => setIsOffRampDialogOpen(false)}
                style={{
                  padding: "0.55rem 0.9rem",
                  border: "1px solid #b9c7d5",
                  borderRadius: "4px",
                  backgroundColor: "#fff",
                  color: "#4c5a67",
                }}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => createRequest("off")}
                disabled={isSubmitting !== null || !canCreateOffRamp}
                style={{
                  padding: "0.55rem 0.9rem",
                  border: "none",
                  borderRadius: "4px",
                  backgroundColor:
                    isSubmitting !== null || !canCreateOffRamp ? "#8fa7be" : "#4f6f8b",
                  color: "#fff",
                  fontWeight: "bold",
                  cursor: isSubmitting !== null || !canCreateOffRamp ? "not-allowed" : "pointer",
                }}
              >
                {isSubmitting === "off" ? "Creating..." : "Create Off-Ramp Request"}
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}
