// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import type { components } from "@/types/api";

type WalletResponse = components["schemas"]["WalletResponse"];

interface WalletCardProps {
  wallet: WalletResponse;
}

/**
 * Card component displaying a single wallet's information.
 */
export function WalletCard({ wallet }: WalletCardProps) {
  return (
    <div
      style={{
        border: "1px solid #ddd",
        borderRadius: "4px",
        padding: "1rem",
        marginBottom: "1rem",
      }}
    >
      <h3 style={{ margin: "0 0 0.5rem 0" }}>{wallet.label || "Unnamed Wallet"}</h3>
      <dl style={{ margin: 0 }}>
        <dt style={{ fontWeight: "bold", fontSize: "0.875rem", color: "#666" }}>
          Wallet ID
        </dt>
        <dd style={{ margin: "0 0 0.5rem 0", fontFamily: "monospace" }}>
          {wallet.wallet_id}
        </dd>

        <dt style={{ fontWeight: "bold", fontSize: "0.875rem", color: "#666" }}>
          Public Address
        </dt>
        <dd style={{ margin: "0 0 0.5rem 0", fontFamily: "monospace", wordBreak: "break-all" }}>
          {wallet.public_address}
        </dd>

        <dt style={{ fontWeight: "bold", fontSize: "0.875rem", color: "#666" }}>
          Status
        </dt>
        <dd style={{ margin: "0 0 0.5rem 0" }}>
          <span style={{
            padding: "0.125rem 0.5rem",
            borderRadius: "4px",
            fontSize: "0.75rem",
            fontWeight: "bold",
            backgroundColor: wallet.status === "active" ? "#d4edda" : "#fff3cd",
            color: wallet.status === "active" ? "#155724" : "#856404",
          }}>
            {wallet.status}
          </span>
        </dd>

        <dt style={{ fontWeight: "bold", fontSize: "0.875rem", color: "#666" }}>
          Created
        </dt>
        <dd style={{ margin: 0, color: "#666" }}>
          {new Date(wallet.created_at).toLocaleDateString()}
        </dd>
      </dl>
    </div>
  );
}
