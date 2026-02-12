// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useMemo, useState } from "react";

interface PaymentRequestBuilderProps {
  recipientAddress: string;
}

/**
 * Builds a shareable link that pre-fills the send form for pull-style requests.
 */
export function PaymentRequestBuilder({ recipientAddress }: PaymentRequestBuilderProps) {
  const [amount, setAmount] = useState("");
  const [token, setToken] = useState<"native" | "usdc">("native");
  const [note, setNote] = useState("");
  const [copied, setCopied] = useState(false);

  const requestLink = useMemo(() => {
    const params = new URLSearchParams();
    params.set("to", recipientAddress);
    if (amount.trim()) params.set("amount", amount.trim());
    params.set("token", token);
    if (note.trim()) params.set("note", note.trim());

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

      <button
        type="button"
        onClick={copyLink}
        style={{
          marginTop: "0.75rem",
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
    </section>
  );
}
