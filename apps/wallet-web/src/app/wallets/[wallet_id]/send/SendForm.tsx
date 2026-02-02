// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import type {
  EstimateGasRequest,
  EstimateGasResponse,
  SendTransactionRequest,
  SendTransactionResponse,
} from "@/lib/api";

// USDC contract address on Fuji testnet
const USDC_FUJI_ADDRESS = "0x5425890298aed601595a70AB815c96711a31Bc65";

interface SendFormProps {
  walletId: string;
  publicAddress: string;
  walletLabel: string | null;
}

type TransactionState =
  | { step: "form" }
  | { step: "confirm"; gasEstimate: EstimateGasResponse }
  | { step: "sending" }
  | { step: "polling"; txHash: string; explorerUrl: string }
  | { step: "success"; txHash: string; explorerUrl: string; blockNumber?: number }
  | { step: "failed"; txHash: string; explorerUrl: string; error?: string };

/**
 * Send transaction form with gas estimation, confirmation, and status polling.
 *
 * Fixed-size components for faster page loads.
 * Polls every 10 seconds, max 2 minutes, with manual refresh option.
 */
export function SendForm({ walletId, publicAddress, walletLabel }: SendFormProps) {
  const router = useRouter();

  // Form state
  const [toAddress, setToAddress] = useState("");
  const [amount, setAmount] = useState("");
  const [token, setToken] = useState<"native" | "usdc">("native");
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [gasLimitOverride, setGasLimitOverride] = useState("");
  const [priorityFeeOverride, setPriorityFeeOverride] = useState("");

  // Transaction state
  const [txState, setTxState] = useState<TransactionState>({ step: "form" });
  const [error, setError] = useState<string | null>(null);
  const [isEstimating, setIsEstimating] = useState(false);

  // Polling state
  const [pollCount, setPollCount] = useState(0);
  const MAX_POLLS = 12; // 12 polls × 10 seconds = 2 minutes

  // Address validation (0x + 40 hex characters)
  const isValidAddress = (addr: string) => /^0x[a-fA-F0-9]{40}$/.test(addr);

  // Amount validation
  const isValidAmount = (amt: string) => {
    const num = parseFloat(amt);
    return !isNaN(num) && num > 0;
  };

  // Get decimals for current token
  const getDecimals = () => (token === "usdc" ? 6 : 18);

  // Format amount display
  const formatAmount = (amt: string) => {
    const decimals = getDecimals();
    const symbol = token === "usdc" ? "USDC" : "AVAX";
    return `${amt} ${symbol}`;
  };

  // Estimate gas
  const handleEstimate = async () => {
    if (!isValidAddress(toAddress)) {
      setError("Invalid recipient address. Must be 0x followed by 40 hex characters.");
      return;
    }
    if (!isValidAmount(amount)) {
      setError("Invalid amount. Must be a positive number.");
      return;
    }

    setError(null);
    setIsEstimating(true);

    try {
      const request: EstimateGasRequest = {
        to: toAddress,
        amount,
        token: token === "usdc" ? USDC_FUJI_ADDRESS : "native",
        network: "fuji",
      };

      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/estimate`,
        {
          method: "POST",
          credentials: "include",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(request),
        }
      );

      if (response.ok) {
        const gasEstimate: EstimateGasResponse = await response.json();
        setTxState({ step: "confirm", gasEstimate });
      } else {
        const text = await response.text();
        setError(text || `Estimation failed (${response.status})`);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Network error");
    } finally {
      setIsEstimating(false);
    }
  };

  // Send transaction
  const handleSend = async () => {
    if (txState.step !== "confirm") return;

    setTxState({ step: "sending" });
    setError(null);

    try {
      const request: SendTransactionRequest = {
        to: toAddress,
        amount,
        token: token === "usdc" ? USDC_FUJI_ADDRESS : "native",
        network: "fuji",
      };

      // Add overrides if specified
      if (gasLimitOverride) {
        request.gas_limit = gasLimitOverride;
      }
      if (priorityFeeOverride) {
        request.max_priority_fee_per_gas = priorityFeeOverride;
      }

      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/send`,
        {
          method: "POST",
          credentials: "include",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(request),
        }
      );

      if (response.ok) {
        const result: SendTransactionResponse = await response.json();
        setTxState({
          step: "polling",
          txHash: result.tx_hash,
          explorerUrl: result.explorer_url,
        });
        setPollCount(0);
      } else {
        const text = await response.text();
        setTxState({ step: "form" });
        setError(text || `Transaction failed (${response.status})`);
      }
    } catch (err) {
      setTxState({ step: "form" });
      setError(err instanceof Error ? err.message : "Network error");
    }
  };

  // Poll for transaction status
  const pollStatus = useCallback(async () => {
    if (txState.step !== "polling") return;

    try {
      const response = await fetch(
        `/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/transactions/${encodeURIComponent(txState.txHash)}`,
        {
          method: "GET",
          credentials: "include",
        }
      );

      if (response.ok) {
        const status = await response.json();
        if (status.status === "confirmed") {
          setTxState({
            step: "success",
            txHash: txState.txHash,
            explorerUrl: txState.explorerUrl,
            blockNumber: status.block_number,
          });
        } else if (status.status === "failed") {
          setTxState({
            step: "failed",
            txHash: txState.txHash,
            explorerUrl: txState.explorerUrl,
          });
        }
      }
    } catch {
      // Silently retry on network errors
    }

    setPollCount((c) => c + 1);
  }, [txState, walletId]);

  // Auto-poll every 10 seconds
  useEffect(() => {
    if (txState.step !== "polling") return;
    if (pollCount >= MAX_POLLS) return;

    const timer = setTimeout(pollStatus, 10000);
    return () => clearTimeout(timer);
  }, [txState, pollCount, pollStatus]);

  // Poll immediately when entering polling state
  useEffect(() => {
    if (txState.step === "polling" && pollCount === 0) {
      pollStatus();
    }
  }, [txState, pollCount, pollStatus]);

  // Render based on current step
  if (txState.step === "success") {
    return (
      <div
        style={{
          border: "1px solid #28a745",
          borderRadius: "8px",
          padding: "2rem",
          backgroundColor: "#d4edda",
          minHeight: "300px",
        }}
      >
        <h2 style={{ color: "#155724", marginTop: 0 }}>✓ Transaction Confirmed</h2>
        <p style={{ color: "#155724" }}>
          Your transaction has been confirmed on the blockchain.
        </p>
        <dl style={{ color: "#155724" }}>
          <dt style={{ fontWeight: "bold", marginTop: "1rem" }}>Transaction Hash</dt>
          <dd style={{ fontFamily: "monospace", wordBreak: "break-all" }}>
            {txState.txHash}
          </dd>
          {txState.blockNumber && (
            <>
              <dt style={{ fontWeight: "bold", marginTop: "1rem" }}>Block Number</dt>
              <dd>{txState.blockNumber}</dd>
            </>
          )}
        </dl>
        <div style={{ marginTop: "1.5rem", display: "flex", gap: "1rem" }}>
          <a
            href={txState.explorerUrl}
            target="_blank"
            rel="noopener noreferrer"
            style={{
              padding: "0.75rem 1.5rem",
              backgroundColor: "#28a745",
              color: "white",
              borderRadius: "4px",
              textDecoration: "none",
            }}
          >
            View on Explorer
          </a>
          <button
            onClick={() => router.push(`/wallets/${walletId}`)}
            style={{
              padding: "0.75rem 1.5rem",
              backgroundColor: "#6c757d",
              color: "white",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
            }}
          >
            Back to Wallet
          </button>
        </div>
      </div>
    );
  }

  if (txState.step === "failed") {
    return (
      <div
        style={{
          border: "1px solid #dc3545",
          borderRadius: "8px",
          padding: "2rem",
          backgroundColor: "#f8d7da",
          minHeight: "300px",
        }}
      >
        <h2 style={{ color: "#721c24", marginTop: 0 }}>✗ Transaction Failed</h2>
        <p style={{ color: "#721c24" }}>
          The transaction was not successful. This may be due to insufficient gas,
          contract rejection, or network issues.
        </p>
        <p style={{ fontFamily: "monospace", wordBreak: "break-all", color: "#721c24" }}>
          TX: {txState.txHash}
        </p>
        <div style={{ marginTop: "1.5rem", display: "flex", gap: "1rem" }}>
          <a
            href={txState.explorerUrl}
            target="_blank"
            rel="noopener noreferrer"
            style={{
              padding: "0.75rem 1.5rem",
              backgroundColor: "#dc3545",
              color: "white",
              borderRadius: "4px",
              textDecoration: "none",
            }}
          >
            View on Explorer
          </a>
          <button
            onClick={() => {
              setTxState({ step: "form" });
              setError(null);
            }}
            style={{
              padding: "0.75rem 1.5rem",
              backgroundColor: "#6c757d",
              color: "white",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
            }}
          >
            Try Again
          </button>
        </div>
      </div>
    );
  }

  if (txState.step === "polling" || txState.step === "sending") {
    const isPending = txState.step === "sending" || pollCount < MAX_POLLS;

    return (
      <div
        style={{
          border: "1px solid #007bff",
          borderRadius: "8px",
          padding: "2rem",
          backgroundColor: "#cce5ff",
          minHeight: "300px",
        }}
      >
        <h2 style={{ color: "#004085", marginTop: 0 }}>
          {txState.step === "sending" ? "Sending..." : "Transaction Pending"}
        </h2>
        {txState.step === "polling" && (
          <>
            <p style={{ color: "#004085" }}>
              Waiting for confirmation... (checked {pollCount}/{MAX_POLLS} times)
            </p>
            <p
              style={{
                fontFamily: "monospace",
                wordBreak: "break-all",
                color: "#004085",
              }}
            >
              TX: {txState.txHash}
            </p>
            <div style={{ marginTop: "1rem", display: "flex", gap: "1rem" }}>
              <a
                href={txState.explorerUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{
                  padding: "0.5rem 1rem",
                  backgroundColor: "#007bff",
                  color: "white",
                  borderRadius: "4px",
                  textDecoration: "none",
                }}
              >
                View on Explorer
              </a>
              {isPending && (
                <button
                  onClick={pollStatus}
                  style={{
                    padding: "0.5rem 1rem",
                    backgroundColor: "#17a2b8",
                    color: "white",
                    border: "none",
                    borderRadius: "4px",
                    cursor: "pointer",
                  }}
                >
                  Refresh Status
                </button>
              )}
            </div>
            {!isPending && (
              <p style={{ color: "#856404", marginTop: "1rem", backgroundColor: "#fff3cd", padding: "0.5rem", borderRadius: "4px" }}>
                Transaction is taking longer than expected. Check the explorer for status.
              </p>
            )}
          </>
        )}
        {txState.step === "sending" && (
          <p style={{ color: "#004085" }}>
            Signing and broadcasting transaction...
          </p>
        )}
      </div>
    );
  }

  if (txState.step === "confirm") {
    const { gasEstimate } = txState;
    return (
      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: "8px",
          padding: "1.5rem",
          minHeight: "400px",
        }}
      >
        <h2 style={{ marginTop: 0 }}>Confirm Transaction</h2>

        <dl>
          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>From</dt>
          <dd style={{ fontFamily: "monospace", fontSize: "0.875rem" }}>
            {publicAddress}
          </dd>

          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>To</dt>
          <dd style={{ fontFamily: "monospace", fontSize: "0.875rem" }}>{toAddress}</dd>

          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>Amount</dt>
          <dd style={{ fontSize: "1.25rem", fontWeight: "bold" }}>
            {formatAmount(amount)}
          </dd>

          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
            Estimated Gas Cost
          </dt>
          <dd>{gasEstimate.estimated_cost} AVAX</dd>

          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
            Gas Limit
          </dt>
          <dd>{gasLimitOverride || gasEstimate.gas_limit}</dd>
        </dl>

        {error && (
          <div
            style={{
              padding: "0.75rem",
              backgroundColor: "#fee",
              border: "1px solid #f00",
              borderRadius: "4px",
              color: "#c00",
              marginTop: "1rem",
            }}
          >
            {error}
          </div>
        )}

        <div style={{ marginTop: "1.5rem", display: "flex", gap: "1rem" }}>
          <button
            onClick={handleSend}
            style={{
              flex: 1,
              padding: "1rem",
              backgroundColor: "#28a745",
              color: "white",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
              fontSize: "1rem",
              fontWeight: "bold",
            }}
          >
            Confirm & Send
          </button>
          <button
            onClick={() => {
              setTxState({ step: "form" });
              setError(null);
            }}
            style={{
              padding: "1rem",
              backgroundColor: "#6c757d",
              color: "white",
              border: "none",
              borderRadius: "4px",
              cursor: "pointer",
            }}
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  // Form step
  return (
    <div
      style={{
        border: "1px solid #ddd",
        borderRadius: "8px",
        padding: "1.5rem",
        minHeight: "400px",
      }}
    >
      <div style={{ marginBottom: "1rem", color: "#666" }}>
        Sending from: <strong>{walletLabel || "Wallet"}</strong>
        <div style={{ fontFamily: "monospace", fontSize: "0.75rem" }}>
          {publicAddress}
        </div>
      </div>

      {/* Token Selection */}
      <div style={{ marginBottom: "1.5rem" }}>
        <label style={{ display: "block", fontWeight: "bold", marginBottom: "0.5rem" }}>
          Token
        </label>
        <div style={{ display: "flex", gap: "0.5rem" }}>
          <button
            type="button"
            onClick={() => setToken("native")}
            style={{
              flex: 1,
              padding: "0.75rem",
              border: token === "native" ? "2px solid #007bff" : "1px solid #ddd",
              borderRadius: "4px",
              backgroundColor: token === "native" ? "#e7f1ff" : "white",
              cursor: "pointer",
            }}
          >
            AVAX
          </button>
          <button
            type="button"
            onClick={() => setToken("usdc")}
            style={{
              flex: 1,
              padding: "0.75rem",
              border: token === "usdc" ? "2px solid #007bff" : "1px solid #ddd",
              borderRadius: "4px",
              backgroundColor: token === "usdc" ? "#e7f1ff" : "white",
              cursor: "pointer",
            }}
          >
            USDC
          </button>
        </div>
      </div>

      {/* Recipient Address */}
      <div style={{ marginBottom: "1.5rem" }}>
        <label
          htmlFor="toAddress"
          style={{ display: "block", fontWeight: "bold", marginBottom: "0.5rem" }}
        >
          Recipient Address
        </label>
        <input
          id="toAddress"
          type="text"
          value={toAddress}
          onChange={(e) => setToAddress(e.target.value)}
          placeholder="0x..."
          style={{
            width: "100%",
            padding: "0.75rem",
            border: "1px solid #ddd",
            borderRadius: "4px",
            fontFamily: "monospace",
            boxSizing: "border-box",
          }}
        />
        {toAddress && !isValidAddress(toAddress) && (
          <div style={{ color: "#dc3545", fontSize: "0.875rem", marginTop: "0.25rem" }}>
            Invalid address format
          </div>
        )}
      </div>

      {/* Amount */}
      <div style={{ marginBottom: "1.5rem" }}>
        <label
          htmlFor="amount"
          style={{ display: "block", fontWeight: "bold", marginBottom: "0.5rem" }}
        >
          Amount ({token === "usdc" ? "USDC" : "AVAX"})
        </label>
        <input
          id="amount"
          type="text"
          value={amount}
          onChange={(e) => setAmount(e.target.value)}
          placeholder="0.0"
          style={{
            width: "100%",
            padding: "0.75rem",
            border: "1px solid #ddd",
            borderRadius: "4px",
            fontSize: "1.25rem",
            boxSizing: "border-box",
          }}
        />
      </div>

      {/* Advanced Options */}
      <div style={{ marginBottom: "1.5rem" }}>
        <button
          type="button"
          onClick={() => setShowAdvanced(!showAdvanced)}
          style={{
            background: "none",
            border: "none",
            color: "#007bff",
            cursor: "pointer",
            padding: 0,
          }}
        >
          {showAdvanced ? "▼ Hide" : "▶ Show"} Advanced Options
        </button>
        {showAdvanced && (
          <div
            style={{
              marginTop: "1rem",
              padding: "1rem",
              backgroundColor: "#f8f9fa",
              borderRadius: "4px",
            }}
          >
            <div style={{ marginBottom: "1rem" }}>
              <label
                htmlFor="gasLimit"
                style={{ display: "block", fontSize: "0.875rem", marginBottom: "0.25rem" }}
              >
                Gas Limit Override (optional)
              </label>
              <input
                id="gasLimit"
                type="text"
                value={gasLimitOverride}
                onChange={(e) => setGasLimitOverride(e.target.value)}
                placeholder="e.g., 21000"
                style={{
                  width: "100%",
                  padding: "0.5rem",
                  border: "1px solid #ddd",
                  borderRadius: "4px",
                  boxSizing: "border-box",
                }}
              />
            </div>
            <div>
              <label
                htmlFor="priorityFee"
                style={{ display: "block", fontSize: "0.875rem", marginBottom: "0.25rem" }}
              >
                Priority Fee Override (wei, optional)
              </label>
              <input
                id="priorityFee"
                type="text"
                value={priorityFeeOverride}
                onChange={(e) => setPriorityFeeOverride(e.target.value)}
                placeholder="e.g., 1500000000"
                style={{
                  width: "100%",
                  padding: "0.5rem",
                  border: "1px solid #ddd",
                  borderRadius: "4px",
                  boxSizing: "border-box",
                }}
              />
            </div>
          </div>
        )}
      </div>

      {error && (
        <div
          style={{
            padding: "0.75rem",
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

      {/* Submit */}
      <button
        type="button"
        onClick={handleEstimate}
        disabled={isEstimating || !toAddress || !amount}
        style={{
          width: "100%",
          padding: "1rem",
          backgroundColor:
            isEstimating || !toAddress || !amount ? "#ccc" : "#007bff",
          color: "white",
          border: "none",
          borderRadius: "4px",
          cursor: isEstimating || !toAddress || !amount ? "not-allowed" : "pointer",
          fontSize: "1rem",
          fontWeight: "bold",
        }}
      >
        {isEstimating ? "Estimating..." : "Review Transaction"}
      </button>

      {/* Faucet links */}
      <div
        style={{
          marginTop: "1.5rem",
          padding: "1rem",
          backgroundColor: "#f8f9fa",
          borderRadius: "4px",
          fontSize: "0.875rem",
          color: "#666",
        }}
      >
        <strong>Need testnet funds?</strong>
        <ul style={{ margin: "0.5rem 0 0 1.5rem", padding: 0 }}>
          <li>
            <a
              href="https://core.app/tools/testnet-faucet/?subnet=c&token=c"
              target="_blank"
              rel="noopener noreferrer"
            >
              Avalanche Fuji Faucet (AVAX)
            </a>
          </li>
          <li>
            <a
              href="https://faucet.circle.com/"
              target="_blank"
              rel="noopener noreferrer"
            >
              Circle Faucet (USDC)
            </a>
          </li>
        </ul>
      </div>
    </div>
  );
}
