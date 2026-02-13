// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";

interface WalletActionsProps {
  walletId: string;
  walletLabel: string | null;
  walletStatus: string;
}

/**
 * Client component for wallet actions (delete, etc.)
 */
export function WalletActions({ walletId, walletLabel, walletStatus }: WalletActionsProps) {
  const router = useRouter();
  const [isDeleting, setIsDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showConfirm, setShowConfirm] = useState(false);

  const handleDelete = async () => {
    setIsDeleting(true);
    setError(null);

    try {
      console.debug("[wallet.actions] deleting wallet", { walletId });
      const response = await fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}`, {
        method: "DELETE",
        credentials: "include",
      });

      if (response.ok) {
        router.push("/wallets");
        router.refresh();
        return;
      }

      const text = await response.text();
      setError(`Failed to delete wallet: ${text || response.statusText}`);
      setIsDeleting(false);
      setShowConfirm(false);
    } catch (err) {
      setError(`Network error: ${err instanceof Error ? err.message : "Unknown error"}`);
      setIsDeleting(false);
      setShowConfirm(false);
    }
  };

  if (walletStatus === "deleted") {
    return <p className="helper-text">This wallet has already been deleted.</p>;
  }

  return (
    <div className="page-row" style={{ gap: "0.6rem" }}>
      {error && <div className="alert error">{error}</div>}

      {!showConfirm ? (
        <div style={{ display: "flex", gap: "0.55rem", flexWrap: "wrap" }}>
          <button className="btn btn-danger" onClick={() => setShowConfirm(true)}>
            Delete wallet
          </button>
          <p className="helper-text" style={{ margin: 0, alignSelf: "center" }}>
            Deletion is soft-delete for operational recovery.
          </p>
        </div>
      ) : (
        <div className="alert error" style={{ background: "#fff8ef", borderColor: "#f1c483", color: "#8a4c08" }}>
          <p style={{ margin: "0 0 0.65rem 0", fontWeight: 700 }}>
            Confirm wallet deletion
          </p>
          <p style={{ margin: "0 0 0.65rem 0" }}>
            Wallet: <strong>{walletLabel || walletId}</strong>
          </p>
          <div style={{ display: "flex", gap: "0.55rem", flexWrap: "wrap" }}>
            <button className="btn btn-danger" onClick={handleDelete} disabled={isDeleting}>
              {isDeleting ? "Deleting..." : "Yes, delete"}
            </button>
            <button className="btn btn-ghost" onClick={() => setShowConfirm(false)} disabled={isDeleting}>
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
