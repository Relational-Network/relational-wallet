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
import { hashEmail, maskEmail } from "@/lib/emailHash";
import {
  recipientMatchesQuery,
  type RecipientShortcut,
} from "@/lib/recipients";

const REUR_FUJI_ADDRESS = "0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63";

interface SendFormProps {
  walletId: string;
  publicAddress: string;
  walletLabel: string | null;
  prefillWarnings?: string[];
  shortcuts?: RecipientShortcut[];
  shortcutsLoadError?: string | null;
  prefill?: {
    recipientType?: "address" | "email";
    to?: string;
    to_email_hash?: string;
    email_display?: string;
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

function tokenTicker(token: "native" | "reur"): string {
  if (token === "native") return "AVAX";
  return "rEUR";
}

function tokenAddress(token: "native" | "reur"): string {
  if (token === "native") return "native";
  return REUR_FUJI_ADDRESS;
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
  const prefillRecipientMode =
    prefill?.recipientType ?? (prefill?.to_email_hash ? "email" : "address");
  const prefillAmount = prefill?.amount ?? "";
  const prefillToken =
    prefill?.token === "reur" ? "reur" : "native";

  const [toAddress, setToAddress] = useState(prefillTo);
  const [recipientMode, setRecipientMode] = useState<"address" | "email">(prefillRecipientMode);
  const [toEmail, setToEmail] = useState("");
  const [emailHash, setEmailHash] = useState<string | null>(prefill?.to_email_hash ?? null);
  const [emailResolved, setEmailResolved] = useState<boolean | null>(
    prefill?.to_email_hash ? true : null
  );
  const [emailDisplay, setEmailDisplay] = useState<string | null>(prefill?.email_display ?? null);
  const [emailChecking, setEmailChecking] = useState(false);
  const [amount, setAmount] = useState(prefillAmount);
  const [token, setToken] = useState<"native" | "reur">(prefillToken);
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

  const MAX_POLLS = 30;

  useEffect(() => {
    setSavedRecipients(shortcuts);
  }, [shortcuts]);

  const handleSaveRecipient = async () => {
    const name = saveRecipientName.trim();

    setSaveRecipientMessage(null);

    if (!name) {
      setSaveRecipientMessage("Recipient name is required.");
      return;
    }

    if (recipientMode === "address") {
      const normalized = toAddress.trim();
      if (!isValidAddress(normalized)) {
        setSaveRecipientMessage("Enter a valid recipient address first.");
        return;
      }

      const existing = savedRecipients.find(
        (recipient) =>
          recipient.recipientType === "address" &&
          recipient.address.toLowerCase() === normalized.toLowerCase()
      );
      if (existing) {
        setSaveRecipientMessage(`Already saved as "${existing.name}".`);
        return;
      }
    } else {
      if (!emailResolved || !emailHash || !emailDisplay) {
        setSaveRecipientMessage("Verify the email recipient before saving it.");
        return;
      }

      const existing = savedRecipients.find(
        (recipient) =>
          recipient.recipientType === "email" &&
          recipient.emailHash.toLowerCase() === emailHash.toLowerCase()
      );
      if (existing) {
        setSaveRecipientMessage(`Already saved as "${existing.name}".`);
        return;
      }
    }

    setIsSavingRecipient(true);

    try {
      const response = await fetch("/api/proxy/v1/bookmarks", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(
          recipientMode === "address"
            ? {
                wallet_id: walletId,
                name,
                recipient_type: "address",
                address: toAddress.trim(),
              }
            : {
                wallet_id: walletId,
                name,
                recipient_type: "email",
                email_hash: emailHash,
                email_display: emailDisplay,
              }
        ),
      });

      if (!response.ok) {
        const text = await response.text();
        setSaveRecipientMessage(text || `Failed to save recipient (${response.status})`);
        return;
      }

      const payload = await response.json();
      const savedShortcut: RecipientShortcut =
        payload.recipient_type === "email"
          ? {
              id: payload.id,
              name: payload.name,
              recipientType: "email",
              emailHash: payload.email_hash,
              emailDisplay: payload.email_display,
            }
          : {
              id: payload.id,
              name: payload.name,
              recipientType: "address",
              address: payload.address,
            };
      setSavedRecipients((current) => [...current, savedShortcut]);
      setShowSaveRecipient(false);
      setSaveRecipientName("");
      setSaveRecipientMessage(`Saved "${savedShortcut.name}".`);
    } catch (saveError) {
      setSaveRecipientMessage(
        saveError instanceof Error ? saveError.message : "Unable to save recipient"
      );
    } finally {
      setIsSavingRecipient(false);
    }
  };

  const handleCheckEmail = async () => {
    const email = toEmail.trim();
    if (!email || !email.includes("@")) {
      setError("Enter a valid email address.");
      return;
    }
    setEmailChecking(true);
    setError(null);
    try {
      const hash = await hashEmail(email);
      setEmailHash(hash);
      const response = await fetch("/api/proxy/v1/resolve/email", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email_hash: hash }),
      });
      if (response.ok) {
        const data = await response.json();
        setEmailResolved(data.found);
        if (!data.found) {
          setEmailDisplay(null);
          setError("No wallet found for this email address.");
        } else {
          setEmailDisplay(maskEmail(email));
        }
      } else {
        setEmailResolved(null);
        setEmailDisplay(null);
        setError("Failed to check email.");
      }
    } catch {
      setEmailResolved(null);
      setEmailDisplay(null);
      setError("Failed to check email.");
    } finally {
      setEmailChecking(false);
    }
  };

  const handleEstimate = async () => {
    if (recipientMode === "address") {
      if (!isValidAddress(toAddress)) {
        setError("Recipient address must be a valid 0x address.");
        return;
      }
    } else {
      if (!emailHash || !emailResolved) {
        setError("Please enter an email and verify the recipient first.");
        return;
      }
    }

    if (!isValidAmount(amount)) {
      setError("Enter a valid positive amount.");
      return;
    }

    setError(null);
    setIsEstimating(true);

    try {
      const request: EstimateGasRequest =
        recipientMode === "address"
          ? {
              to: toAddress.trim(),
              amount: normalizeAmount(amount.trim()),
              token: tokenAddress(token),
              network: "fuji",
            }
          : {
              to_email_hash: emailHash!,
              amount: normalizeAmount(amount.trim()),
              token: tokenAddress(token),
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
      const request: SendTransactionRequest =
        recipientMode === "address"
          ? {
              to: toAddress.trim(),
              amount: normalizeAmount(amount.trim()),
              token: tokenAddress(token),
              network: "fuji",
              gas_limit: gasLimitOverride.trim() || undefined,
              max_priority_fee_per_gas: priorityFeeOverride.trim() || undefined,
            }
          : {
              to_email_hash: emailHash!,
              amount: normalizeAmount(amount.trim()),
              token: tokenAddress(token),
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
          // Delay the callback slightly so the backend has time to
          // index the confirmed transaction before the dashboard re-fetches.
          setTimeout(() => onComplete?.(), 1500);
          return;
        }

        if (status.status === "failed") {
          setTxState({
            step: "failed",
            txHash: txState.txHash,
            explorerUrl: txState.explorerUrl,
          });
          setTimeout(() => onComplete?.(), 1500);
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
    }, pollCount === 0 ? 500 : 1500);

    return () => clearTimeout(timer);
  }, [txState, pollCount, pollStatus]);

  /* ── Success ─────────────────────────────────────────────────────── */

  if (txState.step === "success") {
    return (
      <div className={`stack send-step-shell centered${mode === "dialog" ? " dialog" : ""}`}>
        <div className="send-step-card send-status-content" style={{ textAlign: "center" }}>
          <div style={{
            width: 56, height: 56, borderRadius: "50%", margin: "0 auto 1rem",
            background: "var(--success-light)", display: "grid", placeItems: "center",
          }}>
            <span style={{ fontSize: "1.5rem" }}>✓</span>
          </div>
          <h3 style={{ margin: 0 }}>Transaction confirmed</h3>
          <p className="text-muted" style={{ margin: "0.5rem 0 0" }}>
            {amount} {tokenTicker(token)} sent successfully
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
      <div className={`stack send-step-shell centered${mode === "dialog" ? " dialog" : ""}`}>
        <div className="send-step-card send-status-content" style={{ textAlign: "center" }}>
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
      <div className={`stack send-step-shell centered${mode === "dialog" ? " dialog" : ""}`}>
        <div className="send-status-panel send-status-content" style={{ textAlign: "center" }}>
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
      </div>
    );
  }

  /* ── Confirm ─────────────────────────────────────────────────────── */

  if (txState.step === "confirm") {
    const { gasEstimate } = txState;
    return (
      <div className={`stack send-step-shell centered${mode === "dialog" ? " dialog" : ""}`}>
        <div className="card card-pad send-step-card">
          <h3 className="section-title">Confirm transfer</h3>
          <p className="text-muted" style={{ margin: "0.25rem 0 0" }}>
            From {walletLabel || "Wallet"}
          </p>
          <span className="mono-sm">{publicAddress}</span>

          <div className="stack-sm" style={{ marginTop: "1rem" }}>
            <div className="row-between">
              <span className="text-secondary">To</span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: "0.8125rem" }}>
                {recipientMode === "email" ? emailDisplay ?? maskEmail(toEmail) : shortenAddr(toAddress)}
              </span>
            </div>
            <hr className="divider" />
            <div className="row-between">
              <span className="text-secondary">Amount</span>
              <strong>{amount} {tokenTicker(token)}</strong>
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
    <div className={`stack send-step-shell${mode === "dialog" ? " dialog" : ""}`}>
      <div
        className="stack send-form-scroll"
        style={{ maxWidth: mode === "dialog" ? "100%" : "28rem", margin: "0 auto" }}
      >
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

        {/* Recipient mode toggle */}
        <div className="row" style={{ gap: "0.375rem" }}>
          <button
            type="button"
            className={`chip${recipientMode === "address" ? " active" : ""}`}
            onClick={() => { setRecipientMode("address"); setError(null); }}
          >
            Address
          </button>
          <button
            type="button"
            className={`chip${recipientMode === "email" ? " active" : ""}`}
            onClick={() => { setRecipientMode("email"); setError(null); }}
          >
            Email
          </button>
        </div>

        {recipientMode === "address" ? (
          <>
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
          </>
        ) : (
          <>
            {emailDisplay && !toEmail.trim() ? (
              <div className="card" style={{ padding: "0.875rem", background: "var(--bg-subtle)" }}>
                <div style={{ fontWeight: 600, marginBottom: "0.25rem" }}>Verified recipient</div>
                <div className="text-secondary" style={{ fontSize: "0.875rem" }}>{emailDisplay}</div>
                <button
                  type="button"
                  className="btn btn-ghost"
                  style={{ marginTop: "0.75rem" }}
                  onClick={() => {
                    setToEmail("");
                    setEmailHash(null);
                    setEmailDisplay(null);
                    setEmailResolved(null);
                    setError(null);
                  }}
                >
                  Choose a different email
                </button>
              </div>
            ) : (
              <div className="field">
                <label>Recipient email</label>
                <input
                  type="email"
                  value={toEmail}
                  onChange={(event) => {
                    setToEmail(event.target.value);
                    setEmailHash(null);
                    setEmailResolved(null);
                    setEmailDisplay(null);
                  }}
                  placeholder="recipient@example.com"
                />
              </div>
            )}

            <div className="row" style={{ gap: "0.5rem", alignItems: "center" }}>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => void handleCheckEmail()}
                disabled={emailChecking || !toEmail.trim()}
              >
                {emailChecking ? "Checking…" : "Verify recipient"}
              </button>
              <button
                type="button"
                className="btn btn-ghost"
                onClick={() => setShowSaveRecipient((state) => !state)}
              >
                <Bookmark size={15} /> {showSaveRecipient ? "Cancel" : "Save recipient"}
              </button>
              {emailResolved === true && (
                <span style={{ color: "var(--success)", fontSize: "0.8125rem" }}>✓ Wallet found</span>
              )}
              {emailResolved === false && (
                <span style={{ color: "var(--danger)", fontSize: "0.8125rem" }}>✕ Not found</span>
              )}
            </div>
          </>
        )}

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
                  return recipientMatchesQuery(recipient, recipientSearch);
                })
                .map((recipient) => (
                  <button
                    key={recipient.id}
                    type="button"
                    className={`bookmark-contact${
                      recipient.recipientType === "address"
                        ? recipientMode === "address" &&
                          toAddress.toLowerCase() === recipient.address.toLowerCase()
                          ? " active"
                          : ""
                        : recipientMode === "email" &&
                            emailHash?.toLowerCase() === recipient.emailHash.toLowerCase()
                          ? " active"
                          : ""
                    }`}
                    onClick={() => {
                      if (recipient.recipientType === "address") {
                        setRecipientMode("address");
                        setToAddress(recipient.address);
                        setToEmail("");
                        setEmailHash(null);
                        setEmailDisplay(null);
                        setEmailResolved(null);
                      } else {
                        setRecipientMode("email");
                        setToEmail("");
                        setEmailHash(recipient.emailHash);
                        setEmailDisplay(recipient.emailDisplay);
                        setEmailResolved(true);
                      }
                      setError(null);
                    }}
                  >
                    <div className="bookmark-avatar">
                      {recipient.name.charAt(0).toUpperCase()}
                    </div>
                    <div>
                      <div className="bookmark-name">{recipient.name}</div>
                      <div className="bookmark-addr">
                        {recipient.recipientType === "address"
                          ? shortenAddr(recipient.address)
                          : recipient.emailDisplay}
                      </div>
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
            className={`chip${token === "reur" ? " active" : ""}`}
            onClick={() => setToken("reur")}
          >
            rEUR
          </button>
        </div>

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

        {error ? <div className="alert alert-error">{error}</div> : null}
      </div>

      <div className="row send-form-actions" style={{ gap: "0.5rem" }}>
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void handleEstimate()}
          disabled={
            isEstimating ||
            !amount.trim() ||
            (recipientMode === "address" ? !toAddress.trim() : !emailResolved)
          }
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
