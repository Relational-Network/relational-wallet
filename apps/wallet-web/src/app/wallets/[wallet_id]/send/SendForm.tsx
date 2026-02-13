// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Scan, Bookmark, ChevronDown, ChevronUp, ExternalLink } from "lucide-react";
import type {
  EstimateGasRequest,
  EstimateGasResponse,
  SendTransactionRequest,
  SendTransactionResponse,
} from "@/lib/api";
import { RecipientQrScanner } from "@/components/RecipientQrScanner";

const USDC_FUJI_ADDRESS = "0x5425890298aed601595a70AB815c96711a31Bc65";

interface RecipientShortcut {
  id: string;
  name: string;
  address: string;
}

interface SendFormProps {
  walletId: string;
  publicAddress: string;
  walletLabel: string | null;
  prefillWarnings?: string[];
  shortcuts?: RecipientShortcut[];
  shortcutsLoadError?: string | null;
  prefill?: {
    to?: string;
    amount?: string;
    token?: string;
    note?: string;
  };
  mode?: "dialog" | "page";
  onRequestClose?: () => void;
  onComplete?: () => void;
}

type TransactionState =
  | { step: "form" }
  | { step: "confirm"; gasEstimate: EstimateGasResponse }
  | { step: "sending" }
  | { step: "polling"; txHash: string; explorerUrl: string }
  | { step: "success"; txHash: string; explorerUrl: string; blockNumber?: number }
  | { step: "failed"; txHash: string; explorerUrl: string; error?: string };

function isValidAddress(value: string) {
  return /^0x[a-fA-F0-9]{40}$/.test(value.trim());
}

function normalizeAmount(value: string): string {
  return value.replace(",", ".");
}

function isValidAmount(value: string) {
  const parsed = Number.parseFloat(normalizeAmount(value));
  return Number.isFinite(parsed) && parsed > 0;
}

function shortenAddr(value: string) {
  if (value.length <= 16) return value;
  return `${value.slice(0, 6)}\u2026${value.slice(-4)}`;
}

export function SendForm({
  walletId,
  publicAddress,
  walletLabel,
  prefill,
  prefillWarnings = [],
  shortcuts = [],
  shortcutsLoadError = null,
  mode = "page",
  onRequestClose,
  onComplete,
}: SendFormProps) {
  const router = useRouter();

  const prefillTo = prefill?.to ?? "";
  const prefillAmount = prefill?.amount ?? "";
  const prefillToken = prefill?.token === "usdc" ? "usdc" : "native";

  const [toAddress, setToAddress] = useState(prefillTo);
  const [amount, setAmount] = useState(prefillAmount);
  const [token, setToken] = useState<"native" | "usdc">(prefillToken);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [gasLimitOverride, setGasLimitOverride] = useState("");
  const [priorityFeeOverride, setPriorityFeeOverride] = useState("");

  const [savedRecipients, setSavedRecipients] = useState<RecipientShortcut[]>(shortcuts);
  const [showSaveRecipient, setShowSaveRecipient] = useState(false);
  const [saveRecipientName, setSaveRecipientName] = useState("");
  const [isSavingRecipient, setIsSavingRecipient] = useState(false);
  const [saveRecipientMessage, setSaveRecipientMessage] = useState<string | null>(null);

  const [showQrScanner, setShowQrScanner] = useState(false);
  const [recipientSearch, setRecipientSearch] = useState("");
  const [txState, setTxState] = useState<TransactionState>({ step: "form" });
  const [error, setError] = useState<string | null>(null);
  const [isEstimating, setIsEstimating] = useState(false);
  const [pollCount, setPollCount] = useState(0);

  const MAX_POLLS = 12;

  useEffect(() => {
    if (shortcuts.length > 0) setSavedRecipients(shortcuts);
  }, [shortcuts]);

  const handleSaveRecipient = async () => {
    const normalized = toAddress.trim();
    const name = saveRecipientName.trim();

    setSaveRecipientMessage(null);

    if (!isValidAddress(normalized)) {
      setSaveRecipientMessage("Enter a valid recipient address first.");
      return;
    }

    if (!name) {
      setSaveRecipientMessage("Recipient name is required.");
      return;
    }

    const existing = savedRecipients.find(
      (recipient) => recipient.address.toLowerCase() === normalized.toLowerCase()
    );
    if (existing) {
      setSaveRecipientMessage(`Already saved as \"${existing.name}\".`);
      return;
    }

    setIsSavingRecipient(true);

    try {
      const response = await fetch("/api/proxy/v1/bookmarks", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wallet_id: walletId,
          name,
          address: normalized,
        }),
      });

      if (!response.ok) {
        const text = await response.text();
        setSaveRecipientMessage(text || `Failed to save recipient (${response.status})`);
        return;
      }

      const payload = (await response.json()) as RecipientShortcut;
      setSavedRecipients((current) => [...current, payload]);
      setShowSaveRecipient(false);
      setSaveRecipientName("");
      setSaveRecipientMessage(`Saved \"${payload.name}\".`);
    } catch (saveError) {
      setSaveRecipientMessage(
        saveError instanceof Error ? saveError.message : "Unable to save recipient"
      );
    } finally {
      setIsSavingRecipient(false);
    }
  };

  const handleEstimate = async () => {
    if (!isValidAddress(toAddress)) {
      setError("Recipient address must be a valid 0x address.");
      return;
    }

    if (!isValidAmount(amount)) {
      setError("Enter a valid positive amount.");
      return;
    }

    setError(null);
    setIsEstimating(true);

    try {
      const request: EstimateGasRequest = {
        to: toAddress.trim(),
        amount: normalizeAmount(amount.trim()),
        token: token === "usdc" ? USDC_FUJI_ADDRESS : "native",
        network: "fuji",
      };

      const response = await fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/estimate`, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const text = await response.text();
        setError(text || `Estimate failed (${response.status})`);
        return;
      }

      const gasEstimate: EstimateGasResponse = await response.json();
      setTxState({ step: "confirm", gasEstimate });
    } catch (estimateError) {
      setError(estimateError instanceof Error ? estimateError.message : "Estimate failed");
    } finally {
      setIsEstimating(false);
    }
  };

  const handleSend = async () => {
    if (txState.step !== "confirm") return;

    setTxState({ step: "sending" });
    setError(null);

    try {
      const request: SendTransactionRequest = {
        to: toAddress.trim(),
        amount: normalizeAmount(amount.trim()),
        token: token === "usdc" ? USDC_FUJI_ADDRESS : "native",
        network: "fuji",
        gas_limit: gasLimitOverride.trim() || undefined,
        max_priority_fee_per_gas: priorityFeeOverride.trim() || undefined,
      };

      const response = await fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/send`, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request),
      });

      if (!response.ok) {
        const text = await response.text();
        setTxState({ step: "form" });
        setError(text || `Send failed (${response.status})`);
        return;
      }

      const payload: SendTransactionResponse = await response.json();
      setTxState({
        step: "polling",
        txHash: payload.tx_hash,
        explorerUrl: payload.explorer_url,
      });
      setPollCount(0);
    } catch (sendError) {
      setTxState({ step: "form" });
      setError(sendError instanceof Error ? sendError.message : "Send failed");
    }
  };

  const pollStatus = useCallback(async () => {
    if (txState.step !== "polling") return;

    try {
      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/transactions/${encodeURIComponent(txState.txHash)}`,
        {
          method: "GET",
          credentials: "include",
        }
      );

      if (response.ok) {
        const status = await response.json();
        if (status.status === "confirmed") {
          setTxState({
            step: "success",
            txHash: txState.txHash,
            explorerUrl: txState.explorerUrl,
            blockNumber: status.block_number,
          });
          onComplete?.();
          return;
        }

        if (status.status === "failed") {
          setTxState({
            step: "failed",
            txHash: txState.txHash,
            explorerUrl: txState.explorerUrl,
          });
          onComplete?.();
          return;
        }
      }
    } catch {
      // Retry on next tick.
    }

    setPollCount((count) => count + 1);
  }, [onComplete, txState, walletId]);

  useEffect(() => {
    if (txState.step !== "polling") return;
    if (pollCount >= MAX_POLLS) return;

    const timer = setTimeout(() => {
      void pollStatus();
    }, pollCount === 0 ? 1000 : 9000);

    return () => clearTimeout(timer);
  }, [txState, pollCount, pollStatus]);

  /* ── Success ─────────────────────────────────────────────────────── */

  if (txState.step === "success") {
    return (
      <div className="stack" style={{ minHeight: "16rem" }}>
        <div style={{ textAlign: "center", padding: "1.5rem 0 0.5rem" }}>
          <div style={{
            width: 56, height: 56, borderRadius: "50%", margin: "0 auto 1rem",
            background: "var(--success-light)", display: "grid", placeItems: "center",
          }}>
            <span style={{ fontSize: "1.5rem" }}>✓</span>
          </div>
          <h3 style={{ margin: 0 }}>Transaction confirmed</h3>
          <p className="text-muted" style={{ margin: "0.5rem 0 0" }}>
            {amount} {token === "usdc" ? "USDC" : "AVAX"} sent successfully
          </p>
          <span className="mono-sm" style={{ display: "block", marginTop: "0.5rem", wordBreak: "break-all" }}>{txState.txHash}</span>
          {txState.blockNumber ? (
            <p className="text-muted" style={{ margin: "0.375rem 0 0", fontSize: "0.75rem" }}>Block #{txState.blockNumber}</p>
          ) : null}
        </div>
        <div className="row" style={{ gap: "0.5rem", flexWrap: "wrap", justifyContent: "center" }}>
          <a href={txState.explorerUrl} target="_blank" rel="noopener noreferrer" className="btn btn-secondary">
            <ExternalLink size={14} /> View on Explorer
          </a>
          {mode === "dialog" ? (
            <button type="button" className="btn btn-ghost" onClick={onRequestClose}>Close</button>
          ) : (
            <button type="button" className="btn btn-ghost" onClick={() => router.push(`/wallets/${walletId}`)}>Back to wallet</button>
          )}
        </div>
      </div>
    );
  }

  /* ── Failed ──────────────────────────────────────────────────────── */

  if (txState.step === "failed") {
    return (
      <div className="stack" style={{ minHeight: "16rem" }}>
        <div style={{ textAlign: "center", padding: "1.5rem 0 0.5rem" }}>
          <div style={{
            width: 56, height: 56, borderRadius: "50%", margin: "0 auto 1rem",
            background: "var(--danger-light)", display: "grid", placeItems: "center",
          }}>
            <span style={{ fontSize: "1.5rem" }}>✕</span>
          </div>
          <h3 style={{ margin: 0 }}>Transaction failed</h3>
          <span className="mono-sm" style={{ display: "block", marginTop: "0.5rem", wordBreak: "break-all" }}>{txState.txHash}</span>
        </div>
        <div className="row" style={{ gap: "0.5rem", flexWrap: "wrap", justifyContent: "center" }}>
          <a href={txState.explorerUrl} target="_blank" rel="noopener noreferrer" className="btn btn-secondary">
            <ExternalLink size={14} /> View on Explorer
          </a>
          <button type="button" className="btn btn-ghost" onClick={() => setTxState({ step: "form" })}>Try again</button>
        </div>
      </div>
    );
  }

  /* ── Sending / Polling ───────────────────────────────────────────── */

  if (txState.step === "polling" || txState.step === "sending") {
    return (
      <div className="stack" style={{ minHeight: "16rem", alignItems: "center", justifyContent: "center", textAlign: "center" }}>
        <div className="send-spinner" />
        <h3 style={{ margin: 0 }}>{txState.step === "sending" ? "Signing & broadcasting…" : "Waiting for confirmation…"}</h3>
        <p className="text-muted" style={{ margin: "0.25rem 0 0" }}>
          {txState.step === "sending"
            ? "Your transaction is being signed inside the enclave."
            : `Checking network status (${pollCount}/${MAX_POLLS})`}
        </p>
        {txState.step === "polling" ? (
          <span className="mono-sm" style={{ marginTop: "0.25rem", wordBreak: "break-all" }}>{txState.txHash}</span>
        ) : null}
      </div>
    );
  }

  /* ── Confirm ─────────────────────────────────────────────────────── */

  if (txState.step === "confirm") {
    const { gasEstimate } = txState;
    return (
      <div className="stack">
        <div className="card card-pad">
          <h3 className="section-title">Confirm transfer</h3>
          <p className="text-muted" style={{ margin: "0.25rem 0 0" }}>
            From {walletLabel || "Wallet"}
          </p>
          <span className="mono-sm">{publicAddress}</span>

          <div className="stack-sm" style={{ marginTop: "1rem" }}>
            <div className="row-between">
              <span className="text-secondary">To</span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "0.8125rem" }}>{shortenAddr(toAddress)}</span>
            </div>
            <hr className="divider" />
            <div className="row-between">
              <span className="text-secondary">Amount</span>
              <strong>{amount} {token === "usdc" ? "USDC" : "AVAX"}</strong>
            </div>
            <hr className="divider" />
            <div className="row-between">
              <span className="text-secondary">Network fee</span>
              <span>{gasEstimate.estimated_cost} AVAX</span>
            </div>
          </div>
        </div>

        {error ? <div className="alert alert-error">{error}</div> : null}

        <div className="row" style={{ gap: "0.5rem" }}>
          <button type="button" className="btn btn-primary" onClick={() => void handleSend()} style={{ flex: 1 }}>
            Confirm &amp; send
          </button>
          <button type="button" className="btn btn-secondary" onClick={() => setTxState({ step: "form" })}>
            Back
          </button>
        </div>
      </div>
    );
  }

  /* ── Main form ───────────────────────────────────────────────────── */

  return (
    <div className="stack" style={{ maxWidth: "28rem", margin: "0 auto" }}>
      <div className="text-muted" style={{ display: "flex", alignItems: "center", gap: "0.375rem", flexWrap: "wrap", justifyContent: "center" }}>
        From <strong style={{ color: "var(--ink)" }}>{walletLabel || "Wallet"}</strong>
        <span style={{ fontFamily: "var(--font-mono)", fontSize: "0.75rem" }}>
          {shortenAddr(publicAddress)}
        </span>
      </div>

      {prefillWarnings.length > 0 ? (
        <div className="alert alert-warning">
          Link fields were adjusted.
          <ul style={{ margin: "0.25rem 0 0", paddingLeft: "1.25rem" }}>
            {prefillWarnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {/* ── Recipient ─────────────────────────────────────────────── */}

      <div className="field">
        <label>Recipient address</label>
        <input
          value={toAddress}
          onChange={(event) => setToAddress(event.target.value)}
          placeholder="0x\u2026"
          style={{ fontFamily: "var(--font-mono)" }}
        />
      </div>

      <div className="row" style={{ gap: "0.5rem", flexWrap: "wrap" }}>
        <button type="button" className="btn btn-secondary" onClick={() => setShowQrScanner(true)} style={{ flex: 1 }}>
          <Scan size={16} /> Scan QR code
        </button>
        <button type="button" className="btn btn-ghost" onClick={() => setShowSaveRecipient((state) => !state)}>
          <Bookmark size={15} /> {showSaveRecipient ? "Cancel" : "Save recipient"}
        </button>
      </div>

      {shortcutsLoadError ? <div className="alert alert-warning">{shortcutsLoadError}</div> : null}

      {savedRecipients.length > 0 ? (
        <div className="stack-sm">
          {savedRecipients.length >= 4 ? (
            <input
              value={recipientSearch}
              onChange={(event) => setRecipientSearch(event.target.value)}
              placeholder="Search saved recipients…"
              style={{ fontSize: "0.8125rem" }}
            />
          ) : null}
          <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", maxHeight: "12rem", overflowY: "auto" }}>
            {savedRecipients
              .filter((recipient) => {
                if (!recipientSearch.trim()) return true;
                const query = recipientSearch.toLowerCase();
                return (
                  recipient.name.toLowerCase().includes(query) ||
                  recipient.address.toLowerCase().includes(query)
                );
              })
              .map((recipient) => (
            <button
              key={recipient.id}
              type="button"
              className={`bookmark-contact${toAddress.toLowerCase() === recipient.address.toLowerCase() ? " active" : ""}`}
              onClick={() => {
                setToAddress(recipient.address);
                setError(null);
              }}
            >
              <div className="bookmark-avatar">
                {recipient.name.charAt(0).toUpperCase()}
              </div>
              <div>
                <div className="bookmark-name">{recipient.name}</div>
                <div className="bookmark-addr">{shortenAddr(recipient.address)}</div>
              </div>
            </button>
          ))}
          </div>
        </div>
      ) : null}

      {showSaveRecipient ? (
        <div className="inline-form">
          <input
            value={saveRecipientName}
            onChange={(event) => setSaveRecipientName(event.target.value)}
            placeholder="Recipient name"
            className="input"
          />
          <button
            type="button"
            className="btn btn-secondary"
            onClick={() => void handleSaveRecipient()}
            disabled={isSavingRecipient}
          >
            {isSavingRecipient ? "Saving\u2026" : "Save"}
          </button>
        </div>
      ) : null}

      {saveRecipientMessage ? <p className="text-muted" style={{ margin: 0 }}>{saveRecipientMessage}</p> : null}

      {/* ── Amount ────────────────────────────────────────────────── */}

      <div className="field">
        <label>Amount</label>
        <input
          value={amount}
          onChange={(event) => setAmount(event.target.value)}
          placeholder="0.0"
          inputMode="decimal"
        />
      </div>

      <div className="row" style={{ gap: "0.375rem" }}>
        <button
          type="button"
          className={`chip${token === "native" ? " active" : ""}`}
          onClick={() => setToken("native")}
        >
          AVAX
        </button>
        <button
          type="button"
          className={`chip${token === "usdc" ? " active" : ""}`}
          onClick={() => setToken("usdc")}
        >
          USDC
        </button>
      </div>

      {/* ── Advanced ──────────────────────────────────────────────── */}

      <button
        type="button"
        className="btn btn-ghost"
        onClick={() => setShowAdvanced((state) => !state)}
        style={{ justifyContent: "flex-start" }}
      >
        {showAdvanced ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
        {showAdvanced ? "Hide advanced" : "Advanced options"}
      </button>

      {showAdvanced ? (
        <div className="stack">
          <div className="field">
            <label>Gas limit override</label>
            <input
              value={gasLimitOverride}
              onChange={(event) => setGasLimitOverride(event.target.value)}
              placeholder="21000"
            />
          </div>
          <div className="field">
            <label>Priority fee override (wei)</label>
            <input
              value={priorityFeeOverride}
              onChange={(event) => setPriorityFeeOverride(event.target.value)}
              placeholder="1500000000"
            />
          </div>
        </div>
      ) : null}

      {/* ── Actions ───────────────────────────────────────────────── */}

      {error ? <div className="alert alert-error">{error}</div> : null}

      <div className="row" style={{ gap: "0.5rem" }}>
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void handleEstimate()}
          disabled={isEstimating || !toAddress.trim() || !amount.trim()}
          style={{ flex: 1 }}
        >
          {isEstimating ? "Estimating\u2026" : "Review transaction"}
        </button>
        {mode === "dialog" && onRequestClose ? (
          <button type="button" className="btn btn-secondary" onClick={onRequestClose}>
            Cancel
          </button>
        ) : null}
      </div>

      <RecipientQrScanner
        open={showQrScanner}
        onClose={() => setShowQrScanner(false)}
        onScan={(value) => {
          const maybeAddressMatch = value.match(/0x[a-fA-F0-9]{40}/);
          setToAddress(maybeAddressMatch?.[0] || value.trim());
          setError(null);
        }}
      />
    </div>
  );
}
