// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";

export default function CallbackPage() {
  return (
    <main style={{ maxWidth: "720px", margin: "2rem auto", padding: "0 1rem" }}>
      <h1 style={{ marginBottom: "0.5rem" }}>Bank Authorization Callback</h1>
      <p style={{ color: "#51606e", lineHeight: 1.5 }}>
        Your bank provider redirected back to the app. You can return to your wallet fiat page and
        refresh requests to check the latest on-ramp or off-ramp status.
      </p>
      <div style={{ marginTop: "1rem" }}>
        <Link
          href="/wallets"
          style={{
            display: "inline-block",
            padding: "0.55rem 0.9rem",
            borderRadius: "4px",
            border: "1px solid #b8c6d5",
            color: "#2f4659",
            textDecoration: "none",
          }}
        >
          Back to wallets
        </Link>
      </div>
    </main>
  );
}
