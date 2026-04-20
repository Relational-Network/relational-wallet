// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useCallback, useEffect, useState } from "react";
import { Wallet, RefreshCw, Copy, Check, ExternalLink } from "lucide-react";
import { ActionDialog } from "@/components/ActionDialog";

interface ServiceWalletStatus {
  wallet_id: string;
  public_address: string;
  bootstrapped: boolean;
  chain_network: string;
  reur_contract_address: string;
  avax_balance: string;
  reur_balance: string;
  reur_balance_raw: string;
}

interface ServiceWalletModalProps {
  open: boolean;
  onClose: () => void;
}

function formatBalance(value: string): string {
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) return value;
  return parsed.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: 6,
  });
}

function shortenAddress(address: string): string {
  if (address.length < 16) return address;
  return `${address.slice(0, 10)}...${address.slice(-8)}`;
}

export function ServiceWalletModal({ open, onClose }: ServiceWalletModalProps) {
  const [data, setData] = useState<ServiceWalletStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const fetchStatus = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch("/api/proxy/v1/admin/fiat/service-wallet", {
        method: "GET",
        credentials: "include",
      });
      if (!response.ok) {
        const text = await response.text();
        setError(text || `Failed to load (${response.status})`);
        return;
      }
      const payload: ServiceWalletStatus = await response.json();
      setData(payload);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) void fetchStatus();
  }, [open, fetchStatus]);

  const copyAddress = useCallback(async () => {
    if (!data) return;
    try {
      await navigator.clipboard.writeText(data.public_address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard not available
    }
  }, [data]);

  const avaxLow = data ? Number.parseFloat(data.avax_balance) < 0.01 : false;

  return (
    <ActionDialog open={open} onClose={onClose} title="Service Wallet">
      <div className="stack" style={{ gap: "1rem" }}>
        {error ? (
          <div className="alert alert-error">{error}</div>
        ) : null}

        {loading && !data ? (
          <p className="text-secondary" style={{ textAlign: "center", padding: "1rem 0" }}>
            Loading...
          </p>
        ) : data ? (
          <>
            {/* Address */}
            <div>
              <span className="helper-text">Public Address</span>
              <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginTop: "0.25rem" }}>
                <span className="mono" style={{ fontSize: "0.8125rem", wordBreak: "break-all" }}>
                  {data.public_address}
                </span>
                <button
                  type="button"
                  className="btn btn-ghost"
                  style={{ padding: "0.15rem 0.4rem", flexShrink: 0 }}
                  onClick={() => void copyAddress()}
                  title="Copy address"
                >
                  {copied ? <Check size={14} /> : <Copy size={14} />}
                </button>
              </div>
            </div>

            {/* Balances */}
            <div className="page-grid-2">
              <div className="token-tile">
                <span className="helper-text">AVAX (gas)</span>
                <span className={`token-value${avaxLow ? " insufficient" : ""}`}>
                  {formatBalance(data.avax_balance)}
                </span>
                {avaxLow ? (
                  <span className="helper-text" style={{ color: "var(--warning-600, #b45309)" }}>
                    Low — fund for gas fees
                  </span>
                ) : null}
              </div>
              <div className="token-tile">
                <span className="helper-text">rEUR (reserve)</span>
                <span className="token-value">{formatBalance(data.reur_balance)}</span>
              </div>
            </div>

            {/* Meta */}
            <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.75rem" }}>
                <span className="text-secondary">Network</span>
                <span className="pill">{data.chain_network}</span>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.75rem" }}>
                <span className="text-secondary">rEUR Contract</span>
                <span className="mono" style={{ fontSize: "0.6875rem" }}>
                  {shortenAddress(data.reur_contract_address)}
                </span>
              </div>
              <div style={{ display: "flex", justifyContent: "space-between", fontSize: "0.75rem" }}>
                <span className="text-secondary">Wallet ID</span>
                <span className="mono" style={{ fontSize: "0.6875rem" }}>{data.wallet_id}</span>
              </div>
            </div>

            {/* Faucet links */}
            <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
              <a
                href={`https://core.app/tools/testnet-faucet/?subnet=c&address=${data.public_address}`}
                target="_blank"
                rel="noopener noreferrer"
                className="btn btn-secondary"
                style={{ fontSize: "0.75rem", display: "inline-flex", alignItems: "center", gap: "0.3rem" }}
              >
                <ExternalLink size={12} /> AVAX Faucet
              </a>
              <a
                href={`https://testnet.snowtrace.io/address/${data.public_address}`}
                target="_blank"
                rel="noopener noreferrer"
                className="btn btn-ghost"
                style={{ fontSize: "0.75rem", display: "inline-flex", alignItems: "center", gap: "0.3rem" }}
              >
                <ExternalLink size={12} /> Explorer
              </a>
            </div>
          </>
        ) : null}

        <div className="inline-actions" style={{ justifyContent: "flex-end" }}>
          <button
            type="button"
            className="btn btn-ghost"
            style={{ fontSize: "0.75rem", display: "inline-flex", alignItems: "center", gap: "0.3rem" }}
            onClick={() => void fetchStatus()}
            disabled={loading}
          >
            <RefreshCw size={12} className={loading ? "spin" : ""} /> Refresh
          </button>
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </ActionDialog>
  );
}
