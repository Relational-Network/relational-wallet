// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useEffect, useMemo, useState } from "react";
import { QRCodeSVG } from "qrcode.react";
import { buildPaymentRequestParams } from "@/lib/paymentRequest";

interface PaymentRequestBuilderProps {
  recipientAddress: string;
}

/**
 * Builds a shareable link that pre-fills the send form for pull-style requests.
 */
export function PaymentRequestBuilder({ recipientAddress }: PaymentRequestBuilderProps) {
  const QR_SIZE = 220;
  const [amount, setAmount] = useState("");
  const [token, setToken] = useState<"native" | "usdc">("native");
  const [note, setNote] = useState("");
  const [copied, setCopied] = useState(false);
  const [showQrPopup, setShowQrPopup] = useState(false);

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
    try {
      await navigator.clipboard.writeText(requestLink);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
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

  return (
    <section
      style={{
        border: "1px solid #ddd",
        borderRadius: "4px",
        padding: "1.5rem",
        marginBottom: "1.5rem",
      }}
    >
      <h3 style={{ marginTop: 0, color: "#666" }}>Request Payment (Pull Transfer)</h3>
      <p style={{ marginTop: 0, color: "#666", fontSize: "0.875rem" }}>
        Generate a shareable link that pre-fills the sender&apos;s transfer form.
      </p>

      <div style={{ display: "grid", gap: "0.75rem" }}>
        <div>
          <label htmlFor="requestAmount" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            Amount (optional)
          </label>
          <input
            id="requestAmount"
            type="text"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="e.g. 25"
            style={{
              width: "100%",
              padding: "0.625rem",
              border: "1px solid #ddd",
              borderRadius: "4px",
              boxSizing: "border-box",
            }}
          />
        </div>

        <div>
          <label htmlFor="requestToken" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            Token
          </label>
          <select
            id="requestToken"
            value={token}
            onChange={(e) => setToken(e.target.value === "usdc" ? "usdc" : "native")}
            style={{
              width: "100%",
              padding: "0.625rem",
              border: "1px solid #ddd",
              borderRadius: "4px",
              backgroundColor: "white",
              boxSizing: "border-box",
            }}
          >
            <option value="native">AVAX</option>
            <option value="usdc">USDC</option>
          </select>
        </div>

        <div>
          <label htmlFor="requestNote" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            Note (optional)
          </label>
          <input
            id="requestNote"
            type="text"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="e.g. Grocery split"
            maxLength={140}
            style={{
              width: "100%",
              padding: "0.625rem",
              border: "1px solid #ddd",
              borderRadius: "4px",
              boxSizing: "border-box",
            }}
          />
        </div>

        <div>
          <label htmlFor="requestLink" style={{ display: "block", fontWeight: "bold", marginBottom: "0.25rem" }}>
            Share Link
          </label>
          <textarea
            id="requestLink"
            readOnly
            value={requestLink}
            rows={3}
            style={{
              width: "100%",
              padding: "0.625rem",
              border: "1px solid #ddd",
              borderRadius: "4px",
              fontFamily: "monospace",
              fontSize: "0.8125rem",
              boxSizing: "border-box",
              resize: "vertical",
            }}
          />
        </div>
      </div>

      <div style={{ marginTop: "0.75rem", display: "flex", gap: "0.5rem", flexWrap: "wrap" }}>
        <button
          type="button"
          onClick={copyLink}
          style={{
            padding: "0.5rem 0.9rem",
            border: "1px solid #007bff",
            borderRadius: "4px",
            backgroundColor: copied ? "#28a745" : "#007bff",
            color: "white",
            cursor: "pointer",
          }}
        >
          {copied ? "Copied" : "Copy Link"}
        </button>

        <button
          type="button"
          onClick={() => setShowQrPopup(true)}
          style={{
            padding: "0.5rem 0.9rem",
            border: "1px solid #444",
            borderRadius: "4px",
            backgroundColor: "#fff",
            color: "#222",
            cursor: "pointer",
          }}
        >
          Generate QR
        </button>
      </div>

      {showQrPopup ? (
        <div
          role="dialog"
          aria-modal="true"
          aria-label="Payment request QR code"
          onClick={() => setShowQrPopup(false)}
          style={{
            position: "fixed",
            inset: 0,
            backgroundColor: "rgba(0, 0, 0, 0.55)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 1000,
            padding: "1rem",
          }}
        >
          <div
            onClick={(event) => event.stopPropagation()}
            style={{
              width: "min(420px, 100%)",
              backgroundColor: "#fff",
              borderRadius: "10px",
              border: "1px solid #ddd",
              padding: "1rem",
              boxShadow: "0 20px 40px rgba(0,0,0,0.25)",
            }}
          >
            <h4 style={{ margin: "0 0 0.5rem 0", color: "#222" }}>Scan To Open Payment Link</h4>
            <p style={{ margin: "0 0 0.75rem 0", fontSize: "0.875rem", color: "#666" }}>
              Fixed-size QR code for faster rendering. Press Esc to close.
            </p>
            <div
              style={{
                marginBottom: "0.75rem",
                display: "flex",
                justifyContent: "center",
              }}
            >
              <div
                style={{
                  padding: "0.75rem",
                  border: "1px solid #ddd",
                  borderRadius: "6px",
                  lineHeight: 0,
                  backgroundColor: "#fff",
                }}
              >
                <QRCodeSVG value={requestLink} size={QR_SIZE} level="M" marginSize={2} />
              </div>
            </div>
            <textarea
              readOnly
              value={requestLink}
              rows={2}
              style={{
                width: "100%",
                boxSizing: "border-box",
                padding: "0.5rem",
                borderRadius: "4px",
                border: "1px solid #ddd",
                fontFamily: "monospace",
                fontSize: "0.75rem",
                marginBottom: "0.75rem",
                resize: "none",
              }}
            />
            <div style={{ display: "flex", justifyContent: "flex-end" }}>
              <button
                type="button"
                onClick={() => setShowQrPopup(false)}
                style={{
                  padding: "0.5rem 0.9rem",
                  border: "1px solid #666",
                  borderRadius: "4px",
                  backgroundColor: "#fff",
                  color: "#222",
                  cursor: "pointer",
                }}
              >
                Close
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </section>
  );
}
