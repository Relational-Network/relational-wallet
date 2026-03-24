// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useEffect, useMemo, useState } from "react";
import { QRCodeSVG } from "qrcode.react";

interface PaymentRequestBuilderProps {
  walletId: string;
  compact?: boolean;
}

/**
 * Builds a shareable opaque payment link via the backend API.
 *
 * Instead of encoding the wallet address directly in the URL, we create
 * an opaque token server-side and generate a `/pay?ref=<token>` link.
 */
export function PaymentRequestBuilder({ walletId, compact = false }: PaymentRequestBuilderProps) {
  const QR_SIZE = 220;
  const [amount, setAmount] = useState("");
  const [token, setToken] = useState<"native" | "reur">("native");
  const [note, setNote] = useState("");
  const [copied, setCopied] = useState(false);
  const [showQrPopup, setShowQrPopup] = useState(false);
  const [linkToken, setLinkToken] = useState<string | null>(null);
  const [generating, setGenerating] = useState(false);
  const [genError, setGenError] = useState<string | null>(null);
  const hasAmount = amount.trim().length > 0;

  const requestLink = useMemo(() => {
    if (!linkToken) return "";
    const path = `/pay?ref=${encodeURIComponent(linkToken)}`;
    if (typeof window === "undefined") return path;
    return `${window.location.origin}${path}`;
  }, [linkToken]);

  const generateLink = async () => {
    setGenerating(true);
    setGenError(null);
    try {
      const response = await fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/payment-link`, {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          amount: amount.trim() || undefined,
          token: token === "native" ? undefined : token,
          note: note.trim() || undefined,
          expires_hours: 24,
          single_use: false,
        }),
      });
      if (response.ok) {
        const data = await response.json();
        setLinkToken(data.token);
      } else {
        const text = await response.text();
        setGenError(text || "Failed to create payment link");
      }
    } catch {
      setGenError("Failed to create payment link");
    } finally {
      setGenerating(false);
    }
  };

  // Reset the token when parameters change
  useEffect(() => {
    setLinkToken(null);
  }, [amount, token, note]);

  const copyLink = async () => {
    if (compact && !hasAmount) return;
    // If no opaque token yet, generate one first
    if (!linkToken) {
      await generateLink();
      return;
    }
    try {
      await navigator.clipboard.writeText(requestLink);
      setCopied(true);
      setTimeout(() => setCopied(false), 1400);
    } catch {
      setCopied(false);
    }
  };

  // After generating, auto-copy
  useEffect(() => {
    if (linkToken && !copied) {
      navigator.clipboard.writeText(requestLink).then(() => {
        setCopied(true);
        setTimeout(() => setCopied(false), 1400);
      }).catch(() => {});
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [linkToken]);

  const openQr = async () => {
    if (!linkToken) {
      await generateLink();
    }
    setShowQrPopup(true);
  };

  useEffect(() => {
    if (!showQrPopup) return;

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setShowQrPopup(false);
      }
    };

    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [showQrPopup]);

  const qrPopup = showQrPopup ? (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Payment request QR code"
      onClick={() => setShowQrPopup(false)}
      className="dialog-backdrop"
    >
      <div onClick={(event) => event.stopPropagation()} className="dialog-card" style={{ padding: "1.25rem" }}>
        <h3 style={{ margin: 0 }}>Payment QR</h3>
        <p className="text-muted" style={{ margin: "0.25rem 0 0" }}>Scan to open a prefilled send form.</p>
        <div style={{ display: "grid", placeItems: "center", margin: "1rem 0" }}>
          <div className="qr-frame">
            <QRCodeSVG value={requestLink} size={QR_SIZE} level="M" marginSize={2} />
          </div>
        </div>
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button type="button" className="btn btn-ghost" onClick={() => setShowQrPopup(false)}>Close</button>
        </div>
      </div>
    </div>
  ) : null;

  /* ── Compact mode (inside receive dialog) ────────────────────────── */

  if (compact) {
    return (
      <div className="stack" style={{ width: "100%" }}>
        <p className="text-muted" style={{ margin: 0, textAlign: "center" }}>
          Request a specific amount via shareable payment link.
        </p>

        <div className="grid-2">
          <div className="field">
            <label htmlFor="requestAmount">Amount</label>
            <input
              id="requestAmount"
              type="text"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.0"
              inputMode="decimal"
            />
          </div>
          <div className="field">
            <label htmlFor="requestToken">Token</label>
            <select
              id="requestToken"
              value={token}
              onChange={(e) => setToken(e.target.value === "reur" ? "reur" : "native")}
            >
              <option value="native">AVAX</option>
              <option value="reur">rEUR</option>
            </select>
          </div>
        </div>

        <div className="field">
          <label htmlFor="requestNoteCompact">Note (optional)</label>
          <input
            id="requestNoteCompact"
            type="text"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="e.g. Dinner split"
            maxLength={140}
          />
        </div>

        <div className="row" style={{ gap: "0.5rem", justifyContent: "center" }}>
          <button
            type="button"
            onClick={copyLink}
            className={`btn ${copied ? "btn-ghost" : "btn-primary"}`}
            disabled={!hasAmount || generating}
          >
            {generating ? "Generating…" : copied ? "Copied \u2713" : "Copy payment link"}
          </button>
          <button
            type="button"
            onClick={openQr}
            className="btn btn-secondary"
            disabled={!hasAmount || generating}
          >
            {generating ? "Generating…" : "QR code"}
          </button>
        </div>

        {genError && <p className="text-error" style={{ margin: 0, textAlign: "center", fontSize: "0.875rem" }}>{genError}</p>}

        {qrPopup}
      </div>
    );
  }

  /* ── Full mode (standalone receive page) ─────────────────────────── */

  return (
    <section className="card card-pad">
      <h3 className="section-title">Request payment</h3>
      <p className="text-secondary" style={{ margin: "0.25rem 0 0.75rem" }}>
        Generate a shareable link that prefills the send form for the payer.
      </p>

      <div className="grid-2">
        <div className="field">
          <label htmlFor="requestAmount">Amount (optional)</label>
          <input
            id="requestAmount"
            type="text"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="e.g. 25"
          />
        </div>
        <div className="field">
          <label htmlFor="requestToken">Token</label>
          <select
            id="requestToken"
            value={token}
            onChange={(e) => setToken(e.target.value === "reur" ? "reur" : "native")}
          >
            <option value="native">AVAX</option>
            <option value="reur">rEUR</option>
          </select>
        </div>
      </div>

      <div className="field" style={{ marginTop: "0.75rem" }}>
        <label htmlFor="requestNote">Note (optional)</label>
        <input
          id="requestNote"
          type="text"
          value={note}
          onChange={(e) => setNote(e.target.value)}
          placeholder="e.g. Grocery split"
          maxLength={140}
        />
      </div>

      <div className="row" style={{ marginTop: "0.75rem", gap: "0.5rem" }}>
        <button type="button" onClick={copyLink} className={`btn ${copied ? "btn-ghost" : "btn-primary"}`} disabled={generating}>
          {generating ? "Generating…" : copied ? "Copied \u2713" : "Copy link"}
        </button>
        <button type="button" onClick={openQr} className="btn btn-secondary" disabled={generating}>
          {generating ? "Generating…" : "Generate QR"}
        </button>
      </div>

      {genError && <p className="text-error" style={{ margin: "0.5rem 0 0", fontSize: "0.875rem" }}>{genError}</p>}

      {qrPopup}
    </section>
  );
}
