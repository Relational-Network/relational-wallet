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
      const response = await fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}`, {
        method: "DELETE",
        credentials: "include",
      });

      if (response.ok) {
        // Redirect to wallets list after successful delete
        router.push("/wallets");
        router.refresh();
      } else {
        const text = await response.text();
        setError(`Failed to delete wallet: ${text || response.statusText}`);
        setIsDeleting(false);
        setShowConfirm(false);
      }
    } catch (err) {
      setError(`Network error: ${err instanceof Error ? err.message : "Unknown error"}`);
      setIsDeleting(false);
      setShowConfirm(false);
    }
  };

  // Don't show delete for already deleted wallets
  if (walletStatus === "deleted") {
    return (
      <p style={{ color: "#666" }}>
        This wallet has been deleted.
      </p>
    );
  }

  return (
    <div>
      {error && (
        <div
          style={{
            padding: "0.75rem",
            marginBottom: "1rem",
            backgroundColor: "#fee",
            border: "1px solid #f00",
            borderRadius: "4px",
            color: "#c00",
          }}
        >
          {error}
        </div>
      )}

      {!showConfirm ? (
        <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
          <button
            onClick={() => setShowConfirm(true)}
            style={{
              padding: "0.5rem 1rem",
              backgroundColor: "#dc3545",
              color: "#fff",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
            }}
          >
            Delete Wallet
          </button>
          <p style={{ color: "#666", margin: 0, alignSelf: "center" }}>
            Additional wallet actions (send, receive, transactions) coming soon.
          </p>
        </div>
      ) : (
        <div
          style={{
            padding: "1rem",
            backgroundColor: "#fff3cd",
            border: "1px solid #ffc107",
            borderRadius: "4px",
          }}
        >
          <p style={{ margin: "0 0 1rem 0", color: "#856404", fontWeight: "bold" }}>
            ⚠️ Are you sure you want to delete this wallet?
          </p>
          <p style={{ margin: "0 0 1rem 0", color: "#856404" }}>
            Wallet: <strong>{walletLabel || walletId}</strong>
          </p>
          <p style={{ margin: "0 0 1rem 0", color: "#856404", fontSize: "0.875rem" }}>
            This action will mark the wallet as deleted. The wallet data will be retained for recovery purposes.
          </p>
          <div style={{ display: "flex", gap: "0.5rem" }}>
            <button
              onClick={handleDelete}
              disabled={isDeleting}
              style={{
                padding: "0.5rem 1rem",
                backgroundColor: isDeleting ? "#999" : "#dc3545",
                color: "#fff",
                border: "none",
                borderRadius: "4px",
                cursor: isDeleting ? "not-allowed" : "pointer",
              }}
            >
              {isDeleting ? "Deleting..." : "Yes, Delete Wallet"}
            </button>
            <button
              onClick={() => setShowConfirm(false)}
              disabled={isDeleting}
              style={{
                padding: "0.5rem 1rem",
                backgroundColor: "#6c757d",
                color: "#fff",
                border: "none",
                borderRadius: "4px",
                cursor: isDeleting ? "not-allowed" : "pointer",
              }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
