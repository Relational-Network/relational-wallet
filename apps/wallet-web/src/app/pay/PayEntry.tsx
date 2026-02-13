// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import type { WalletResponse } from "@/lib/api";
import type { ParsedPaymentRequest } from "@/lib/paymentRequest";
import { SendForm } from "@/app/wallets/[wallet_id]/send/SendForm";

interface PayEntryProps {
  wallets: WalletResponse[];
  prefill: ParsedPaymentRequest;
  warnings: string[];
}

export function PayEntry({ wallets, prefill, warnings }: PayEntryProps) {
  const [selectedWalletId, setSelectedWalletId] = useState<string>(wallets[0]?.wallet_id || "");
  const [to, setTo] = useState(prefill.to ?? "");
  const [amount, setAmount] = useState(prefill.amount ?? "");
  const [token, setToken] = useState<"native" | "usdc">(prefill.token);
  const [note, setNote] = useState(prefill.note ?? "");
  const [showSendForm, setShowSendForm] = useState(false);

  const canContinue = useMemo(
    () => Boolean(selectedWalletId && to.trim()),
    [selectedWalletId, to]
  );

  const selectedWallet = wallets.find((w) => w.wallet_id === selectedWalletId);

  const openSendFlow = () => {
    if (!canContinue) return;
    setShowSendForm(true);
  };

  const shortenAddr = (addr: string) =>
    addr.length > 14 ? `${addr.slice(0, 6)}\u2026${addr.slice(-4)}` : addr;

  if (showSendForm && selectedWallet) {
    return (
      <main className="pay-layout">
        <div className="pay-container">
          <div className="row" style={{ justifyContent: "flex-start" }}>
            <Link href="/wallets" className="btn btn-ghost">← Back to wallets</Link>
          </div>
          <div style={{ textAlign: "center", marginBottom: "0.25rem" }}>
            <span className="badge badge-brand">Payment Request</span>
          </div>
          <div className="card card-pad">
            <SendForm
              walletId={selectedWallet.wallet_id}
              publicAddress={selectedWallet.public_address || ""}
              walletLabel={selectedWallet.label ?? null}
              prefill={{
                to: to.trim(),
                amount: amount.trim() || undefined,
                token,
                note: note.trim() || undefined,
              }}
              mode="dialog"
              onRequestClose={() => setShowSendForm(false)}
            />
          </div>
          <footer className="landing-footer">
            © {new Date().getFullYear()} Relational Network ·{" "}
            <a href="https://github.com/Relational-Network/relational-wallet" target="_blank" rel="noopener noreferrer">
              Source code (AGPL-3.0)
            </a>
          </footer>
        </div>
      </main>
    );
  }

  return (
    <main className="pay-layout">
      <div className="pay-container">
        <div className="row" style={{ justifyContent: "flex-start" }}>
          <Link href="/wallets" className="btn btn-ghost">← Back to wallets</Link>
        </div>
        <div style={{ textAlign: "center", marginBottom: "0.25rem" }}>
          <span className="badge badge-brand">Payment Request</span>
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

              <hr className="divider" />

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

        <footer className="landing-footer">
          © {new Date().getFullYear()} Relational Network ·{" "}
          <a href="https://github.com/Relational-Network/relational-wallet" target="_blank" rel="noopener noreferrer">
            Source code (AGPL-3.0)
          </a>
        </footer>
      </div>

    </main>
  );
}
