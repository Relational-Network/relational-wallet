// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState } from "react";
import type { WalletResponse } from "@/lib/api";

interface ManageWalletsSheetProps {
  wallets: WalletResponse[];
  selectedWalletId: string | null;
  onSelectWallet: (walletId: string) => void;
  onCreateWallet: (label?: string) => Promise<void>;
}

export function ManageWalletsSheet({
  wallets,
  selectedWalletId,
  onSelectWallet,
  onCreateWallet,
}: ManageWalletsSheetProps) {
  const [label, setLabel] = useState("");
  const [loading, setLoading] = useState(false);

  const createWallet = async () => {
    setLoading(true);
    try {
      await onCreateWallet(label.trim() || undefined);
      setLabel("");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="stack">
      <section>
        <h3 className="section-title" style={{ marginBottom: "0.5rem" }}>Your wallets</h3>
        <div className="stack-sm">
          {wallets.map((wallet) => {
            const addr = wallet.public_address || "";
            const short = addr.length > 14 ? `${addr.slice(0, 6)}…${addr.slice(-4)}` : addr;
            return (
              <button
                key={wallet.wallet_id}
                type="button"
                className={`wallet-list-item${selectedWalletId === wallet.wallet_id ? " active" : ""}`}
                onClick={() => onSelectWallet(wallet.wallet_id)}
              >
                <div className="bookmark-avatar">
                  {(wallet.label || "W").charAt(0).toUpperCase()}
                </div>
                <div style={{ flex: 1, minWidth: 0, textAlign: "left" }}>
                  <div className="bookmark-name">{wallet.label || "Wallet"}</div>
                  <div className="bookmark-addr">{short}</div>
                </div>
                {wallet.status !== "active" && (
                  <span className="badge badge-warning">{wallet.status}</span>
                )}
                {selectedWalletId === wallet.wallet_id && (
                  <span className="badge badge-brand">Active</span>
                )}
              </button>
            );
          })}
        </div>
      </section>

      <hr className="divider" />

      <section>
        <h3 className="section-title" style={{ marginBottom: "0.5rem" }}>Create wallet</h3>
        <div className="inline-form">
          <input
            className="input"
            value={label}
            onChange={(event) => setLabel(event.target.value)}
            placeholder="Optional label"
            disabled={loading}
          />
          <button type="button" className="btn btn-primary" onClick={() => void createWallet()} disabled={loading}>
            {loading ? "Creating…" : "Create"}
          </button>
        </div>
      </section>
    </div>
  );
}
