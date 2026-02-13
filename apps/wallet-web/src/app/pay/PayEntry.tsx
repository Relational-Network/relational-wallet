// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";
import type { WalletResponse } from "@/lib/api";
import { RecipientQrScanner } from "@/components/RecipientQrScanner";
import { buildPaymentRequestParams, type ParsedPaymentRequest } from "@/lib/paymentRequest";

interface PayEntryProps {
  wallets: WalletResponse[];
  prefill: ParsedPaymentRequest;
  warnings: string[];
}

export function PayEntry({ wallets, prefill, warnings }: PayEntryProps) {
  const router = useRouter();

  const [selectedWalletId, setSelectedWalletId] = useState<string>(wallets[0]?.wallet_id || "");
  const [to, setTo] = useState(prefill.to ?? "");
  const [amount, setAmount] = useState(prefill.amount ?? "");
  const [token, setToken] = useState<"native" | "usdc">(prefill.token);
  const [note, setNote] = useState(prefill.note ?? "");
  const [showScanner, setShowScanner] = useState(false);

  const canContinue = useMemo(
    () => Boolean(selectedWalletId && to.trim()),
    [selectedWalletId, to]
  );

  const selectedWallet = wallets.find((w) => w.wallet_id === selectedWalletId);

  const openSendFlow = () => {
    if (!canContinue) return;

    const params = buildPaymentRequestParams(
      {
        to: to.trim(),
        amount: amount.trim() || undefined,
        token,
        note: note.trim() || undefined,
      },
      { includeDefaultToken: true }
    );

    router.push(`/wallets/${encodeURIComponent(selectedWalletId)}/send?${params.toString()}`);
  };

  const shortenAddr = (addr: string) =>
    addr.length > 14 ? `${addr.slice(0, 6)}\u2026${addr.slice(-4)}` : addr;

  return (
    <main className="pay-layout">
      <div className="pay-container">
        <div style={{ textAlign: "center", marginBottom: "0.25rem" }}>
          <span className="badge badge-brand">Payment Request</span>
          <h1 style={{ margin: "0.75rem 0 0.25rem", fontSize: "1.5rem", fontWeight: 800, letterSpacing: "-0.02em" }}>
            Pay someone
          </h1>
        </div>

        {warnings.length > 0 ? (
          <div className="alert alert-warning">
            Link details adjusted:
            <ul style={{ margin: "0.25rem 0 0", paddingLeft: "1.25rem" }}>
              {warnings.map((warning) => (
                <li key={warning}>{warning}</li>
              ))}
            </ul>
          </div>
        ) : null}

        {wallets.length === 0 ? (
          <div className="card card-pad empty-state">
            <h3>No wallet available</h3>
            <p>Create a wallet first from your dashboard.</p>
          </div>
        ) : (
          <div className="card card-pad">
            <div className="stack">
              <div className="field">
                <label>Pay from wallet</label>
                <select value={selectedWalletId} onChange={(event) => setSelectedWalletId(event.target.value)}>
                  {wallets.map((wallet) => {
                    const addr = wallet.public_address || "";
                    return (
                      <option value={wallet.wallet_id} key={wallet.wallet_id}>
                        {wallet.label || "Wallet"} ({shortenAddr(addr)})
                      </option>
                    );
                  })}
                </select>
              </div>

              {selectedWallet ? (
                <div className="mono-sm" style={{ marginTop: "-0.375rem" }}>
                  {selectedWallet.public_address}
                </div>
              ) : null}

              <hr className="divider" />

              <div className="field">
                <label>Recipient address</label>
                <input
                  value={to}
                  onChange={(event) => setTo(event.target.value)}
                  placeholder="0x\u2026"
                  style={{ fontFamily: "var(--font-mono)" }}
                />
              </div>

              <div className="row" style={{ gap: "0.375rem" }}>
                <button type="button" className="btn btn-ghost" onClick={() => setShowScanner(true)}>
                  Scan QR
                </button>
              </div>

              <div className="field">
                <label>Amount (optional)</label>
                <input
                  value={amount}
                  onChange={(event) => setAmount(event.target.value)}
                  placeholder="0.0"
                  inputMode="decimal"
                />
              </div>

              <div className="row" style={{ gap: "0.375rem" }}>
                <button
                  type="button"
                  className={`chip${token === "native" ? " active" : ""}`}
                  onClick={() => setToken("native")}
                >
                  AVAX
                </button>
                <button
                  type="button"
                  className={`chip${token === "usdc" ? " active" : ""}`}
                  onClick={() => setToken("usdc")}
                >
                  USDC
                </button>
              </div>

              <div className="field">
                <label>Note (optional)</label>
                <input
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                  placeholder="Payment note"
                />
              </div>

              <button
                type="button"
                className="btn btn-primary"
                onClick={openSendFlow}
                disabled={!canContinue}
                style={{ marginTop: "0.5rem" }}
              >
                Continue to send
              </button>
            </div>
          </div>
        )}

        <footer className="app-footer">
          \u00a9 {new Date().getFullYear()} Relational Network \u00b7{" "}
          <a href="https://github.com/Relational-Network/relational-wallet" target="_blank" rel="noopener noreferrer">
            Open source \u00b7 AGPL-3.0
          </a>
        </footer>
      </div>

      <RecipientQrScanner
        open={showScanner}
        onClose={() => setShowScanner(false)}
        onScan={(value) => {
          const maybeAddressMatch = value.match(/0x[a-fA-F0-9]{40}/);
          setTo(maybeAddressMatch?.[0] || value.trim());
        }}
      />
    </main>
  );
}
