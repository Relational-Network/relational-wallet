// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useEffect, useCallback } from "react";

/**
 * Token balance information from the backend.
 */
export interface TokenBalance {
  symbol: string;
  name: string;
  balance_raw: string;
  balance_formatted: string;
  decimals: number;
  contract_address: string | null;
}

/**
 * Full balance response from the backend.
 * Matches the BalanceResponse struct in rust-server.
 */
export interface BalanceResponse {
  wallet_id: string;
  address: string;
  network: string;
  chain_id: number;
  native_balance: TokenBalance;
  token_balances: TokenBalance[];
}

interface WalletBalanceProps {
  walletId: string;
  publicAddress: string;
  walletStatus: string;
}

/**
 * Client component for displaying wallet balance with refresh capability.
 *
 * Fetches native AVAX balance and ERC-20 token balances (USDC) from the
 * Avalanche Fuji testnet via the backend balance endpoint.
 */
export function WalletBalance({
  walletId,
  publicAddress,
  walletStatus,
}: WalletBalanceProps) {
  const [balance, setBalance] = useState<BalanceResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const fetchBalance = useCallback(async () => {
    // Don't fetch balance for deleted or suspended wallets
    if (walletStatus === "deleted" || walletStatus === "suspended") {
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      // Fetch full balance including ERC-20 tokens
      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/balance?network=fuji`,
        {
          method: "GET",
          credentials: "include",
        }
      );

      if (response.ok) {
        const data: BalanceResponse = await response.json();
        setBalance(data);
        setLastUpdated(new Date());
      } else if (response.status === 503) {
        setError("Blockchain network unavailable. Please try again later.");
      } else {
        const text = await response.text();
        setError(text || `Failed to fetch balance (${response.status})`);
      }
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Network error fetching balance"
      );
    } finally {
      setIsLoading(false);
    }
  }, [walletId, walletStatus]);

  // Fetch balance on mount
  useEffect(() => {
    fetchBalance();
  }, [fetchBalance]);

  // Don't show balance section for deleted wallets
  if (walletStatus === "deleted") {
    return null;
  }

  // Suspended wallet message
  if (walletStatus === "suspended") {
    return (
      <section
        style={{
          border: "1px solid #ddd",
          borderRadius: "4px",
          padding: "1.5rem",
          marginBottom: "2rem",
          backgroundColor: "#fff3cd",
        }}
      >
        <h2 style={{ marginTop: 0, color: "#856404" }}>Balance</h2>
        <p style={{ margin: 0, color: "#856404" }}>
          Balance queries are disabled for suspended wallets.
        </p>
      </section>
    );
  }

  return (
    <section
      style={{
        border: "1px solid #ddd",
        borderRadius: "4px",
        padding: "1.5rem",
        marginBottom: "2rem",
        backgroundColor: "#ffffff",
      }}
    >
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "1rem",
        }}
      >
        <h2 style={{ margin: 0, color: "#1a1a1a" }}>Balance</h2>
        <button
          onClick={fetchBalance}
          disabled={isLoading}
          style={{
            padding: "0.5rem 1rem",
            backgroundColor: isLoading ? "#ccc" : "#007bff",
            color: "#fff",
            border: "none",
            borderRadius: "4px",
            cursor: isLoading ? "not-allowed" : "pointer",
            display: "flex",
            alignItems: "center",
            gap: "0.5rem",
          }}
          title="Refresh balance"
        >
          {isLoading ? (
            <>
              <span
                style={{
                  display: "inline-block",
                  width: "14px",
                  height: "14px",
                  border: "2px solid #fff",
                  borderTopColor: "transparent",
                  borderRadius: "50%",
                  animation: "spin 1s linear infinite",
                }}
              />
              Refreshing...
            </>
          ) : (
            <>↻ Refresh</>
          )}
        </button>
      </div>

      {/* Network indicator */}
      <div style={{ marginBottom: "1rem" }}>
        <span
          style={{
            padding: "0.25rem 0.5rem",
            backgroundColor: "#e7f1ff",
            color: "#0066cc",
            borderRadius: "4px",
            fontSize: "0.75rem",
            fontWeight: "bold",
          }}
        >
          🔷 Fuji Testnet
        </span>
      </div>

      {error ? (
        <div
          style={{
            padding: "1rem",
            backgroundColor: "#ffebee",
            border: "1px solid #ffcdd2",
            borderRadius: "4px",
            color: "#c62828",
          }}
        >
          <strong>Error:</strong> {error}
          <button
            onClick={fetchBalance}
            style={{
              marginLeft: "1rem",
              padding: "0.25rem 0.5rem",
              backgroundColor: "#c62828",
              color: "#fff",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
              fontSize: "0.875rem",
            }}
          >
            Retry
          </button>
        </div>
      ) : isLoading && !balance ? (
        <div
          style={{
            padding: "2rem",
            textAlign: "center",
            color: "#666",
          }}
        >
          <div
            style={{
              display: "inline-block",
              width: "24px",
              height: "24px",
              border: "3px solid #ddd",
              borderTopColor: "#007bff",
              borderRadius: "50%",
              animation: "spin 1s linear infinite",
            }}
          />
          <p style={{ margin: "1rem 0 0 0", color: "#666" }}>
            Loading balance...
          </p>
        </div>
      ) : balance ? (
        <div>
          {/* Native AVAX balance */}
          <div
            style={{
              padding: "1rem",
              backgroundColor: "#f8f9fa",
              borderRadius: "8px",
              marginBottom: "1rem",
              border: "1px solid #e9ecef",
            }}
          >
            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: "0.5rem",
                marginBottom: "0.5rem",
              }}
            >
              <span style={{ fontSize: "1.5rem" }}>🔺</span>
              <span style={{ fontWeight: "bold", color: "#1a1a1a" }}>AVAX</span>
              <span style={{ color: "#666", fontSize: "0.875rem" }}>
                Native Token
              </span>
            </div>
            <div
              style={{
                fontSize: "2rem",
                fontWeight: "bold",
                color: "#1a1a1a",
                fontFamily: "monospace",
              }}
            >
              {formatBalance(balance.native_balance.balance_formatted)}
            </div>
            <div
              style={{ fontSize: "0.75rem", color: "#666", marginTop: "0.25rem" }}
            >
              {balance.native_balance.balance_raw} wei
            </div>
          </div>

          {/* ERC-20 Token balances */}
          {balance.token_balances.length > 0 && (
            <div>
              <h3
                style={{
                  margin: "1rem 0 0.5rem 0",
                  fontSize: "1rem",
                  color: "#1a1a1a",
                }}
              >
                Tokens
              </h3>
              {balance.token_balances.map((token, index) => (
                <div
                  key={token.contract_address || index}
                  style={{
                    padding: "1rem",
                    backgroundColor: "#f8f9fa",
                    borderRadius: "8px",
                    marginBottom: "0.5rem",
                    border: "1px solid #e9ecef",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "0.5rem",
                      marginBottom: "0.5rem",
                    }}
                  >
                    <span style={{ fontSize: "1.25rem" }}>
                      {token.symbol === "USDC" ? "💵" : "🪙"}
                    </span>
                    <span style={{ fontWeight: "bold", color: "#1a1a1a" }}>
                      {token.symbol}
                    </span>
                    <span style={{ color: "#666", fontSize: "0.875rem" }}>
                      {token.name}
                    </span>
                  </div>
                  <div
                    style={{
                      fontSize: "1.5rem",
                      fontWeight: "bold",
                      color: "#1a1a1a",
                      fontFamily: "monospace",
                    }}
                  >
                    {formatBalance(token.balance_formatted)}
                  </div>
                  {token.contract_address && (
                    <div
                      style={{
                        fontSize: "0.7rem",
                        color: "#999",
                        marginTop: "0.25rem",
                        fontFamily: "monospace",
                      }}
                    >
                      {token.contract_address}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          {/* Network details */}
          <dl style={{ margin: "1rem 0 0 0", fontSize: "0.875rem" }}>
            <dt
              style={{ fontWeight: "bold", color: "#666", marginTop: "0.75rem" }}
            >
              Network
            </dt>
            <dd style={{ margin: "0.25rem 0 0 0", color: "#1a1a1a" }}>
              {balance.network} (Chain ID: {balance.chain_id})
            </dd>
          </dl>

          {/* Last updated */}
          {lastUpdated && (
            <p
              style={{
                marginTop: "1rem",
                fontSize: "0.75rem",
                color: "#888",
              }}
            >
              Last updated: {lastUpdated.toLocaleTimeString()}
            </p>
          )}

          {/* Testnet faucet links */}
          <div
            style={{
              marginTop: "1rem",
              padding: "0.75rem",
              backgroundColor: "#e3f2fd",
              borderRadius: "4px",
              fontSize: "0.875rem",
              color: "#1a1a1a",
              border: "1px solid #90caf9",
            }}
          >
            <strong style={{ color: "#1565c0" }}>Need test tokens?</strong>
            <ul style={{ margin: "0.5rem 0 0 0", paddingLeft: "1.25rem" }}>
              <li>
                <a
                  href={`https://core.app/tools/testnet-faucet/?subnet=c&address=${encodeURIComponent(publicAddress)}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: "#1565c0", textDecoration: "underline" }}
                >
                  Get test AVAX from Avalanche Faucet →
                </a>
              </li>
              <li style={{ marginTop: "0.25rem" }}>
                <a
                  href="https://faucet.circle.com/"
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{ color: "#1565c0", textDecoration: "underline" }}
                >
                  Get test USDC from Circle Faucet →
                </a>
              </li>
            </ul>
          </div>
        </div>
      ) : null}

      {/* CSS animation for spinner */}
      <style>{`
        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </section>
  );
}

/**
 * Format a balance string for display.
 * Shows up to 6 decimal places, trimming trailing zeros.
 */
function formatBalance(balance: string): string {
  const num = parseFloat(balance);
  if (isNaN(num)) return balance;

  // Format with up to 6 decimal places, then trim trailing zeros
  const formatted = num.toFixed(6);
  return formatted.replace(/\.?0+$/, "") || "0";
}
