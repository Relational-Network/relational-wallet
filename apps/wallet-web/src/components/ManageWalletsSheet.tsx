// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState } from "react";
import { Trash2 } from "lucide-react";
import type { WalletResponse } from "@/lib/api";

interface ManageWalletsSheetProps {
  wallets: WalletResponse[];
  selectedWalletId: string | null;
  onSelectWallet: (walletId: string) => void;
  onCreateWallet: (label?: string) => Promise<void>;
  onDeleteWallet?: (walletId: string) => Promise<void>;
}

export function ManageWalletsSheet({
  wallets,
  selectedWalletId,
  onSelectWallet,
  onCreateWallet,
  onDeleteWallet,
}: ManageWalletsSheetProps) {
  const [label, setLabel] = useState("");
  const [loading, setLoading] = useState(false);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [confirmName, setConfirmName] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [sheetMessage, setSheetMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);

  const createWallet = async () => {
    setLoading(true);
    setSheetMessage(null);
    try {
      await onCreateWallet(label.trim() || undefined);
      setLabel("");
      setSheetMessage({ type: "success", text: "Wallet created." });
    } catch (err) {
      setSheetMessage({ type: "error", text: err instanceof Error ? err.message : "Create failed" });
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (walletId: string) => {
    if (!onDeleteWallet) return;
    setDeleting(true);
    setSheetMessage(null);
    try {
      await onDeleteWallet(walletId);
      setConfirmDeleteId(null);
      setConfirmName("");
      setSheetMessage({ type: "success", text: "Wallet deleted." });
    } catch (err) {
      setSheetMessage({ type: "error", text: err instanceof Error ? err.message : "Delete failed" });
    } finally {
      setDeleting(false);
    }
  };

  const confirmDeleteWallet = wallets.find((w) => w.wallet_id === confirmDeleteId);
  const confirmDeleteLabel = confirmDeleteWallet?.label || "Wallet";
  const nameMatches = confirmName.trim().toLowerCase() === confirmDeleteLabel.toLowerCase();

  return (
    <div className="stack">
      {sheetMessage ? (
        <div className={`alert ${sheetMessage.type === "success" ? "alert-success" : "alert-error"}`}>
          {sheetMessage.text}
        </div>
      ) : null}

      <section>
        <h3 className="section-title" style={{ marginBottom: "0.5rem" }}>Your wallets</h3>
        <div className="stack-sm">
          {wallets.map((wallet) => {
            const addr = wallet.public_address || "";
            const short = addr.length > 14 ? `${addr.slice(0, 6)}…${addr.slice(-4)}` : addr;

            if (confirmDeleteId === wallet.wallet_id) {
              return (
                <div
                  key={wallet.wallet_id}
                  className="wallet-list-item"
                  style={{ flexDirection: "column", alignItems: "stretch", gap: "0.5rem", cursor: "default" }}
                >
                  <p style={{ margin: 0, textAlign: "center", fontSize: "0.875rem", color: "var(--danger)" }}>
                    ⚠ Delete <strong>{confirmDeleteLabel}</strong>?
                  </p>
                  <p className="text-muted" style={{ margin: 0, textAlign: "center", fontSize: "0.75rem" }}>
                    Type <strong>{confirmDeleteLabel}</strong> to confirm
                  </p>
                  <input
                    className="input"
                    value={confirmName}
                    onChange={(e) => setConfirmName(e.target.value)}
                    placeholder={confirmDeleteLabel}
                    autoFocus
                    disabled={deleting}
                    style={{ textAlign: "center", fontSize: "0.8125rem" }}
                  />
                  <div className="row" style={{ gap: "0.5rem", justifyContent: "center" }}>
                    <button
                      type="button"
                      className="btn btn-danger"
                      onClick={() => void handleDelete(wallet.wallet_id)}
                      disabled={deleting || !nameMatches}
                      style={{ fontSize: "0.8125rem" }}
                    >
                      {deleting ? "Deleting…" : "Delete"}
                    </button>
                    <button
                      type="button"
                      className="btn btn-ghost"
                      onClick={() => { setConfirmDeleteId(null); setConfirmName(""); }}
                      disabled={deleting}
                      style={{ fontSize: "0.8125rem" }}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              );
            }

            return (
              <div
                key={wallet.wallet_id}
                className={`wallet-list-item${selectedWalletId === wallet.wallet_id ? " active" : ""}`}
                style={{ cursor: "pointer" }}
                onClick={() => onSelectWallet(wallet.wallet_id)}
                role="button"
                tabIndex={0}
                onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") onSelectWallet(wallet.wallet_id); }}
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
                {onDeleteWallet ? (
                  <button
                    type="button"
                    onClick={(e) => { e.stopPropagation(); setConfirmDeleteId(wallet.wallet_id); }}
                    style={{ padding: "0.25rem", background: "none", border: "none", cursor: "pointer", color: "var(--ink-muted)", borderRadius: "0.25rem" }}
                    title="Delete wallet"
                  >
                    <Trash2 size={14} />
                  </button>
                ) : null}
              </div>
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
