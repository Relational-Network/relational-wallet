// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useEffect, useMemo, useState } from "react";
import { QRCodeSVG } from "qrcode.react";
import { buildPaymentRequestParams } from "@/lib/paymentRequest";

interface PaymentRequestBuilderProps {
  recipientAddress: string;
  compact?: boolean;
}

/**
 * Builds a shareable link that pre-fills the send form for pull-style requests.
 */
export function PaymentRequestBuilder({ recipientAddress, compact = false }: PaymentRequestBuilderProps) {
  const QR_SIZE = 220;
  const [amount, setAmount] = useState("");
  const [token, setToken] = useState<"native" | "usdc">("native");
  const [note, setNote] = useState("");
  const [copied, setCopied] = useState(false);
  const [showQrPopup, setShowQrPopup] = useState(false);
  const hasAmount = amount.trim().length > 0;

  const requestLink = useMemo(() => {
    const params = buildPaymentRequestParams(
      {
        to: recipientAddress,
        amount: amount.trim() || undefined,
        token,
        note: note.trim() || undefined,
      },
      { includeDefaultToken: true }
    );

    const path = `/pay?${params.toString()}`;
    if (typeof window === "undefined") return path;
    return `${window.location.origin}${path}`;
  }, [amount, note, recipientAddress, token]);

  const copyLink = async () => {
    if (compact && !hasAmount) return;
    try {
      await navigator.clipboard.writeText(requestLink);
      setCopied(true);
      setTimeout(() => setCopied(false), 1400);
    } catch {
      setCopied(false);
    }
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
              onChange={(e) => setToken(e.target.value === "usdc" ? "usdc" : "native")}
            >
              <option value="native">AVAX</option>
              <option value="usdc">USDC</option>
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
            disabled={!hasAmount}
          >
            {copied ? "Copied \u2713" : "Copy payment link"}
          </button>
          <button
            type="button"
            onClick={() => setShowQrPopup(true)}
            className="btn btn-secondary"
            disabled={!hasAmount}
          >
            QR code
          </button>
        </div>

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
            onChange={(e) => setToken(e.target.value === "usdc" ? "usdc" : "native")}
          >
            <option value="native">AVAX</option>
            <option value="usdc">USDC</option>
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
        <button type="button" onClick={copyLink} className={`btn ${copied ? "btn-ghost" : "btn-primary"}`}>
          {copied ? "Copied \u2713" : "Copy link"}
        </button>
        <button type="button" onClick={() => setShowQrPopup(true)} className="btn btn-secondary">
          Generate QR
        </button>
      </div>

      {qrPopup}
    </section>
  );
}
