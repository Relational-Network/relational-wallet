// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useCallback } from "react";
import { Copy, Check } from "lucide-react";
import { toast } from "sonner";

interface CopyAddressProps {
  /** Full Ethereum address */
  address: string;
  /** Optional label shown before the truncated address */
  label?: string;
  /** Show truncated address text (default true) */
  showAddress?: boolean;
}

function truncate(addr: string) {
  if (addr.length <= 12) return addr;
  return `${addr.slice(0, 6)}…${addr.slice(-4)}`;
}

/**
 * Compound copy-address widget.
 * Shows a truncated address with a copy button. Provides toast feedback.
 */
export function CopyAddress({ address, label, showAddress = true }: CopyAddressProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(address);
    } catch {
      /* fallback for insecure contexts */
      const textarea = document.createElement("textarea");
      textarea.value = address;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
    }
    setCopied(true);
    toast.success("Address copied");
    setTimeout(() => setCopied(false), 1800);
  }, [address]);

  if (!showAddress) {
    return (
      <button onClick={handleCopy} className="btn-icon" title="Copy address">
        {copied ? <Check size={16} /> : <Copy size={16} />}
      </button>
    );
  }

  return (
    <div className="address-display">
      {label && <span style={{ fontWeight: 600, fontFamily: "var(--font-sans)" }}>{label}</span>}
      <span className="address-text">{truncate(address)}</span>
      <button onClick={handleCopy} className="btn-icon" title="Copy address" style={{ border: 0, width: 28, height: 28 }}>
        {copied ? <Check size={14} /> : <Copy size={14} />}
      </button>
    </div>
  );
}
