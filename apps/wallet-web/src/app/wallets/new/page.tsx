// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { useAuth } from "@clerk/nextjs";
import { apiClient } from "@/lib/api";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";

/**
 * Fast wallet creation route.
 * Optimized for clean-state onboarding after Clerk signup.
 * Redirects immediately if the user already has a wallet.
 */
export default function NewWalletPage() {
  const router = useRouter();
  const { getToken } = useAuth();

  const [label, setLabel] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [checking, setChecking] = useState(true);

  // Redirect immediately if user already has a wallet
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const token = await getToken();
        const res = await apiClient.listWallets(token || "");
        if (cancelled) return;
        if (res.success && res.data.wallets.length > 0) {
          router.replace(`/wallets/${res.data.wallets[0].wallet_id}`);
          return;
        }
      } catch { /* proceed to show form */ }
      if (!cancelled) setChecking(false);
    })();
    return () => { cancelled = true; };
  }, [getToken, router]);

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault();
    setLoading(true);
    setError(null);

    try {
      const token = await getToken();
      console.debug("[wallets.new] Creating wallet", { hasLabel: Boolean(label.trim()) });

      const response = await apiClient.createWallet(token || "", {
        label: label.trim() || undefined,
      });

      if (response.success) {
        const walletId = response.data.wallet.wallet_id;
        console.debug("[wallets.new] Wallet created", { walletId });
        router.push(`/wallets/${walletId}`);
        return;
      }

      if (response.error.status === 401) {
        router.push("/sign-in");
        return;
      }

      if (response.error.status === 409) {
        setError("You already have a wallet. Only one wallet per account is allowed.");
        // Redirect to wallet list after a brief delay
        setTimeout(() => router.push("/wallets"), 2000);
        return;
      }

      if (response.error.status === 403) {
        setError("Access denied. You do not have permission to create wallets.");
      } else {
        setError(`Unable to create wallet: ${response.error.message}`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unexpected error while creating wallet");
    } finally {
      setLoading(false);
    }
  };

  return (
    <SimpleWalletShell
      topBar={
        <>
          <div className="app-top-left">
            <Link href="/wallets" className="btn btn-ghost">← Back</Link>
            <span style={{ fontWeight: 700 }}>Create wallet</span>
          </div>
        </>
      }
    >
      {checking ? (
        <div className="card card-pad" style={{ maxWidth: "38rem", textAlign: "center" }}>
          <p className="text-muted">Loading…</p>
        </div>
      ) : (
      <div className="card card-pad" style={{ maxWidth: "38rem" }}>
        <form onSubmit={handleSubmit} className="stack">
          <div className="field">
            <label htmlFor="walletLabel">Wallet label (optional)</label>
            <input
              id="walletLabel"
              type="text"
              value={label}
              onChange={(event) => setLabel(event.target.value)}
              placeholder="Primary, treasury, operations..."
              disabled={loading}
            />
          </div>

          {error && <div className="alert alert-error">{error}</div>}

          <div className="row" style={{ gap: "0.5rem" }}>
            <button type="submit" className="btn btn-primary" disabled={loading}>
              {loading ? "Creating…" : "Create wallet"}
            </button>
            <Link href="/wallets" className="btn btn-secondary">
              Cancel
            </Link>
          </div>

          <p className="text-muted" style={{ margin: 0 }}>
            Debug logs are available in browser console while creating wallets.
          </p>
        </form>
      </div>
      )}
    </SimpleWalletShell>
  );
}
