// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useCallback } from "react";

interface CopyAddressProps {
  address: string;
  /** Display label next to the button. */
  label?: string;
}

/**
 * Client component that copies a wallet address to the clipboard.
 *
 * Shows a brief "Copied!" confirmation after clicking.
 */
export function CopyAddress({ address, label }: CopyAddressProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(address);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback for older browsers
      const textarea = document.createElement("textarea");
      textarea.value = address;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }, [address]);

  return (
    <button
      onClick={handleCopy}
      title="Copy address to clipboard"
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        gap: "0.375rem",
        minWidth: "9.5rem",
        padding: "0.375rem 0.75rem",
        border: "1px solid #ccc",
        borderRadius: "4px",
        background: copied ? "#d4edda" : "#f8f9fa",
        color: copied ? "#155724" : "#333",
        cursor: "pointer",
        fontSize: "0.8125rem",
        fontFamily: "inherit",
        transition: "background 0.2s, color 0.2s",
      }}
    >
      {copied ? "✓ Copied!" : `📋 ${label || "Copy Address"}`}
    </button>
  );
}
