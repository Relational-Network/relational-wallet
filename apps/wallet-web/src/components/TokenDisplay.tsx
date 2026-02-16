// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState } from "react";
import { useAuth } from "@clerk/nextjs";

export function TokenDisplay() {
  const { getToken } = useAuth();
  const [copied, setCopied] = useState(false);
  const [token, setToken] = useState<string | null>(null);
  const [showToken, setShowToken] = useState(false);

  const fetchPreferredToken = async () => {
    return (await getToken({ template: "default" })) ?? (await getToken());
  };

  const copyToken = async () => {
    try {
      const jwt = await fetchPreferredToken();
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
      const jwt = await fetchPreferredToken();
      setToken(jwt);
    }
    setShowToken(!showToken);
  };

  return (
    <section className="card pad">
      <h2 className="card-title">JWT Token (Dev Tool)</h2>
      <p className="card-subtitle">Copy token for curl/tests while backend auth is enabled.</p>

      <div className="inline-actions" style={{ marginTop: "0.8rem" }}>
        <button className={`btn ${copied ? "btn-soft" : "btn-primary"}`} onClick={copyToken}>
          {copied ? "Copied" : "Copy JWT token"}
        </button>
        <button className="btn btn-ghost" onClick={toggleShowToken}>
          {showToken ? "Hide token" : "Show token"}
        </button>
      </div>

      {showToken && token ? (
        <pre
          style={{
            marginTop: "0.8rem",
            background: "#0f2433",
            color: "#d8e7f0",
            borderRadius: "12px",
            padding: "0.75rem",
            maxHeight: "220px",
            overflow: "auto",
            fontSize: "0.72rem",
            fontFamily: "var(--font-mono)",
            whiteSpace: "pre-wrap",
            wordBreak: "break-all",
          }}
        >
          {token}
        </pre>
      ) : null}

      <details style={{ marginTop: "0.8rem" }}>
        <summary className="helper-text" style={{ cursor: "pointer" }}>
          Example curl commands
        </summary>
        <pre
          style={{
            marginTop: "0.55rem",
            background: "#0f2433",
            color: "#d8e7f0",
            borderRadius: "12px",
            padding: "0.75rem",
            overflow: "auto",
            fontSize: "0.72rem",
            fontFamily: "var(--font-mono)",
          }}
        >
{`# Export the token
export JWT="<paste token here>"

# List wallets
curl -k -H "Authorization: Bearer $JWT" \\
  https://localhost:8080/v1/wallets

# Get wallet balance
curl -k -H "Authorization: Bearer $JWT" \\
  "https://localhost:8080/v1/wallets/{wallet_id}/balance?network=fuji"`}
        </pre>
      </details>
    </section>
  );
}
