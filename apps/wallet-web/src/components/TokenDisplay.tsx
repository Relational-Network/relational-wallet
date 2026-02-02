// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState } from "react";
import { useAuth } from "@clerk/nextjs";

/**
 * Component to display and copy the current JWT token.
 * Useful for testing API calls with curl or other tools.
 */
export function TokenDisplay() {
  const { getToken } = useAuth();
  const [copied, setCopied] = useState(false);
  const [token, setToken] = useState<string | null>(null);
  const [showToken, setShowToken] = useState(false);

  const copyToken = async () => {
    try {
      const jwt = await getToken();
      if (jwt) {
        await navigator.clipboard.writeText(jwt);
        setToken(jwt);
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    } catch (error) {
      console.error("Failed to copy token:", error);
    }
  };

  const toggleShowToken = async () => {
    if (!showToken && !token) {
      const jwt = await getToken();
      setToken(jwt);
    }
    setShowToken(!showToken);
  };

  return (
    <div
      style={{
        border: "1px solid #ddd",
        borderRadius: "4px",
        padding: "1.5rem",
        marginBottom: "2rem",
        backgroundColor: "#f8f9fa",
      }}
    >
      <h2 style={{ marginTop: 0, marginBottom: "1rem" }}>🔑 JWT Token (Dev Tool)</h2>
      <p style={{ color: "#666", fontSize: "0.875rem", marginBottom: "1rem" }}>
        Use this token to test API endpoints with curl or other HTTP clients.
      </p>

      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem" }}>
        <button
          onClick={copyToken}
          style={{
            padding: "0.5rem 1rem",
            backgroundColor: copied ? "#28a745" : "#007bff",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: "pointer",
            transition: "background-color 0.2s",
          }}
        >
          {copied ? "✓ Copied!" : "📋 Copy JWT Token"}
        </button>
        <button
          onClick={toggleShowToken}
          style={{
            padding: "0.5rem 1rem",
            backgroundColor: "#6c757d",
            color: "white",
            border: "none",
            borderRadius: "4px",
            cursor: "pointer",
          }}
        >
          {showToken ? "🙈 Hide Token" : "👁 Show Token"}
        </button>
      </div>

      {showToken && token && (
        <div
          style={{
            backgroundColor: "#1e1e1e",
            color: "#d4d4d4",
            padding: "1rem",
            borderRadius: "4px",
            fontFamily: "monospace",
            fontSize: "0.75rem",
            wordBreak: "break-all",
            maxHeight: "200px",
            overflow: "auto",
          }}
        >
          {token}
        </div>
      )}

      <details style={{ marginTop: "1rem" }}>
        <summary style={{ cursor: "pointer", color: "#666", fontSize: "0.875rem" }}>
          Example curl commands
        </summary>
        <pre
          style={{
            backgroundColor: "#1e1e1e",
            color: "#d4d4d4",
            padding: "1rem",
            borderRadius: "4px",
            fontSize: "0.75rem",
            overflow: "auto",
            marginTop: "0.5rem",
          }}
        >
{`# Export the token
export JWT="<paste token here>"

# List wallets
curl -k -H "Authorization: Bearer $JWT" \\
  https://localhost:8080/v1/wallets

# Get wallet balance
curl -k -H "Authorization: Bearer $JWT" \\
  "https://localhost:8080/v1/wallets/{wallet_id}/balance?network=fuji"

# List transactions
curl -k -H "Authorization: Bearer $JWT" \\
  "https://localhost:8080/v1/wallets/{wallet_id}/transactions?network=fuji"`}
        </pre>
      </details>
    </div>
  );
}
