// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { CheckCircle2 } from "lucide-react";

/**
 * TrueLayer return URI callback landing page.
 *
 * When opened inside a popup (window.opener exists), it posts a message
 * back to the parent window and auto-closes. Otherwise it renders the
 * standard "Authorization Complete" view.
 */
export default function CallbackPage() {
  // Detect popup after mount to avoid SSR/client hydration mismatch
  // (window.opener is unavailable during SSR).
  const [isPopup, setIsPopup] = useState(false);

  useEffect(() => {
    const popup = typeof window !== "undefined" && !!window.opener;
    setIsPopup(popup);
    if (!popup) return;
    try {
      window.opener.postMessage({ type: "fiat-callback-complete" }, window.location.origin);
    } catch {
      // Cross-origin or opener already closed — user sees fallback UI.
    }
    const timer = setTimeout(() => window.close(), 600);
    return () => clearTimeout(timer);
  }, []);

  if (isPopup) {
    return (
      <main className="callback-layout">
        <div className="card callback-card">
          <div className="callback-icon">
            <CheckCircle2 size={32} />
          </div>
          <h2 style={{ margin: 0, fontSize: "1.25rem", fontWeight: 700 }}>Done</h2>
          <p className="text-secondary" style={{ margin: "0.75rem 0 0" }}>
            This window will close automatically.
          </p>
        </div>
      </main>
    );
  }

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
