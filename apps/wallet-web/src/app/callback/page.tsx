// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { CheckCircle2 } from "lucide-react";

/**
 * TrueLayer return URI callback landing page.
 */
export default function CallbackPage() {
  return (
    <main className="callback-layout">
      <div className="card callback-card">
        <div className="callback-icon">
          <CheckCircle2 size={32} />
        </div>
        <h2 style={{ margin: 0, fontSize: "1.25rem", fontWeight: 700 }}>Authorization Complete</h2>
        <p className="text-secondary" style={{ margin: "0.75rem 0 0" }}>
          Your bank authorization has been received. Return to your wallet to check the status.
        </p>
        <div className="stack" style={{ marginTop: "1.5rem", gap: "0.5rem" }}>
          <Link className="btn btn-primary" href="/wallets">
            Go to Wallet
          </Link>
        </div>
      </div>
    </main>
  );
}
