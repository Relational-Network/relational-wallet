// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { useAuth } from "@clerk/nextjs";
import { apiClient } from "@/lib/api";

/**
 * Create wallet page (authenticated).
 *
 * Allows users to create a new wallet.
 */
export default function NewWalletPage() {
  const router = useRouter();
  const { getToken } = useAuth();

  const [label, setLabel] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    setLoading(true);
    setError(null);

    try {
      const token = await getToken();

      // Create wallet - label is optional per OpenAPI spec
      const response = await apiClient.createWallet(token || "", {
        label: label.trim() || undefined,
      });

      if (response.success) {
        // Redirect to the new wallet's detail page
        router.push(`/wallets/${response.data.wallet.wallet_id}`);
      } else {
        // Handle different error statuses
        if (response.error.status === 401) {
          router.push("/sign-in");
        } else if (response.error.status === 403) {
          setError("Access denied. You do not have permission to create wallets.");
        } else {
          setError(`Unable to create wallet: ${response.error.message}`);
        }
      }
    } catch {
      setError("An unexpected error occurred. Please try again.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <main style={{ padding: "2rem", maxWidth: "600px", margin: "0 auto" }}>
      <header style={{ marginBottom: "2rem" }}>
        <Link href="/wallets" style={{ color: "#666", textDecoration: "none" }}>
          ← Back to Wallets
        </Link>
        <h1>Create New Wallet</h1>
      </header>

      {error && (
        <div
          style={{
            padding: "1rem",
            backgroundColor: "#fee",
            border: "1px solid #f00",
            borderRadius: "4px",
            color: "#c00",
            marginBottom: "1rem",
          }}
        >
          {error}
        </div>
      )}

      <form onSubmit={handleSubmit}>
        <div style={{ marginBottom: "1rem" }}>
          <label htmlFor="label" style={{ display: "block", marginBottom: "0.5rem" }}>
            Wallet Label (optional)
          </label>
          <input
            type="text"
            id="label"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="e.g., Primary Wallet"
            disabled={loading}
            style={{
              width: "100%",
              padding: "0.5rem",
              border: "1px solid #ccc",
              borderRadius: "4px",
              fontSize: "1rem",
            }}
          />
        </div>

        <button
          type="submit"
          disabled={loading}
          style={{
            padding: "0.75rem 1.5rem",
            backgroundColor: loading ? "#999" : "#333",
            color: "#fff",
            border: "none",
            borderRadius: "4px",
            cursor: loading ? "not-allowed" : "pointer",
            fontSize: "1rem",
          }}
        >
          {loading ? "Creating..." : "Create Wallet"}
        </button>
      </form>
    </main>
  );
}
