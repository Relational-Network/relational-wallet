// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";

/**
 * Wallet not found page.
 */
export default function WalletNotFound() {
  return (
    <main style={{ padding: "2rem", maxWidth: "600px", margin: "0 auto", textAlign: "center" }}>
      <h1>Wallet Not Found</h1>
      <p style={{ color: "#666" }}>
        The wallet you are looking for does not exist or you do not have access to it.
      </p>
      <Link
        href="/wallets"
        style={{
          display: "inline-block",
          marginTop: "1rem",
          padding: "0.5rem 1rem",
          backgroundColor: "#333",
          color: "#fff",
          textDecoration: "none",
          borderRadius: "4px",
        }}
      >
        Back to Wallets
      </Link>
    </main>
  );
}
