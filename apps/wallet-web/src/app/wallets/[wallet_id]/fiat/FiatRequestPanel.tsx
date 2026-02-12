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
  const [amountOffRamp, setAmountOffRamp] = useState("10");
  const [note, setNote] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState<"on" | "off" | null>(null);
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
      setNote("");
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
      <h2 style={{ marginTop: 0 }}>Fiat On/Off-Ramp</h2>
      <p style={{ marginTop: 0, color: "#666", fontSize: "0.875rem" }}>
        Creates live sandbox requests via backend provider integration. Production cutover should
        be configuration-only.
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

        <div>
          <label htmlFor="onrampAmount" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            On-Ramp Amount (EUR)
          </label>
          <input
            id="onrampAmount"
            type="text"
            value={amountOnRamp}
            onChange={(event) => setAmountOnRamp(event.target.value)}
            style={{
              width: "100%",
              boxSizing: "border-box",
              padding: "0.6rem",
              borderRadius: "4px",
              border: "1px solid #ddd",
            }}
          />
        </div>

        <div>
          <label htmlFor="offrampAmount" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            Off-Ramp Amount (EUR)
          </label>
          <input
            id="offrampAmount"
            type="text"
            value={amountOffRamp}
            onChange={(event) => setAmountOffRamp(event.target.value)}
            style={{
              width: "100%",
              boxSizing: "border-box",
              padding: "0.6rem",
              borderRadius: "4px",
              border: "1px solid #ddd",
            }}
          />
        </div>

        <div>
          <label htmlFor="fiatNote" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            Note (optional)
          </label>
          <input
            id="fiatNote"
            type="text"
            value={note}
            onChange={(event) => setNote(event.target.value)}
            maxLength={140}
            style={{
              width: "100%",
              boxSizing: "border-box",
              padding: "0.6rem",
              borderRadius: "4px",
              border: "1px solid #ddd",
            }}
          />
        </div>
      </div>

      <div style={{ display: "flex", gap: "0.75rem", flexWrap: "wrap", marginBottom: "1rem" }}>
        <button
          type="button"
          onClick={() => createRequest("on")}
          disabled={isSubmitting !== null || !canCreateOnRamp}
          style={{
            padding: "0.6rem 0.95rem",
            border: "none",
            borderRadius: "4px",
            backgroundColor: isSubmitting !== null || !canCreateOnRamp ? "#84b6dd" : "#0b7bd3",
            color: "#fff",
            cursor: isSubmitting !== null || !canCreateOnRamp ? "not-allowed" : "pointer",
            fontWeight: "bold",
          }}
        >
          {isSubmitting === "on" ? "Creating..." : "Create On-Ramp Request"}
        </button>

        <button
          type="button"
          onClick={() => createRequest("off")}
          disabled={isSubmitting !== null || !canCreateOffRamp}
          style={{
            padding: "0.6rem 0.95rem",
            border: "none",
            borderRadius: "4px",
            backgroundColor: isSubmitting !== null || !canCreateOffRamp ? "#8fa7be" : "#4f6f8b",
            color: "#fff",
            cursor: isSubmitting !== null || !canCreateOffRamp ? "not-allowed" : "pointer",
            fontWeight: "bold",
          }}
        >
          {isSubmitting === "off" ? "Creating..." : "Create Off-Ramp Request"}
        </button>

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
    </section>
  );
}
