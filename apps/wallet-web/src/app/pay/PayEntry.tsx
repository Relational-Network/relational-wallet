// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import type { WalletResponse } from "@/lib/api";
import type { ParsedPaymentRequest } from "@/lib/paymentRequest";
import { SendForm } from "@/app/wallets/[wallet_id]/send/SendForm";

interface PayEntryProps {
  wallets: WalletResponse[];
  prefill: ParsedPaymentRequest;
  warnings: string[];
}

export function PayEntry({ wallets, prefill: initialPrefill, warnings: initialWarnings }: PayEntryProps) {
  const [selectedWalletId, setSelectedWalletId] = useState<string>(wallets[0]?.wallet_id || "");
  const [warnings, setWarnings] = useState(initialWarnings);
  const [recipientType, setRecipientType] = useState<"address" | "email">(
    initialPrefill.recipientType ?? (initialPrefill.to_email_hash ? "email" : "address")
  );
  const [to, setTo] = useState(initialPrefill.to ?? "");
  const [toEmailHash, setToEmailHash] = useState(initialPrefill.to_email_hash ?? "");
  const [emailDisplay, setEmailDisplay] = useState(initialPrefill.email_display ?? "");
  const [amount, setAmount] = useState(initialPrefill.amount ?? "");
  const [token, setToken] = useState<"native" | "reur">(initialPrefill.token);
  const [note, setNote] = useState(initialPrefill.note ?? "");
  const [showSendForm, setShowSendForm] = useState(false);
  const [resolving, setResolving] = useState(false);

  // Resolve opaque payment-link ref token on mount
  useEffect(() => {
    if (!initialPrefill.ref) return;
    let cancelled = false;
    setResolving(true);
    fetch(`/api/proxy/v1/payment-link/${encodeURIComponent(initialPrefill.ref)}`, {
      method: "GET",
      credentials: "include",
    })
      .then(async (res) => {
        if (cancelled) return;
        if (res.ok) {
          const data = await res.json();
          const resolvedAmount = data.amount ?? "";
          const resolvedToken = data.token_type === "reur" ? "reur" as const : "native" as const;
          const resolvedNote = data.note ?? "";
          if (data.recipient_type === "email" && data.to_email_hash && data.email_display) {
            setRecipientType("email");
            setTo("");
            setToEmailHash(data.to_email_hash);
            setEmailDisplay(data.email_display);
          } else {
            const resolvedTo = data.public_address ?? "";
            setRecipientType("address");
            setTo(resolvedTo);
            setToEmailHash("");
            setEmailDisplay("");
          }
          setAmount(resolvedAmount);
          setToken(resolvedToken);
          setNote(resolvedNote);
        } else {
          setWarnings((w) => [...w, "Payment link not found or expired."]);
        }
      })
      .catch(() => {
        if (!cancelled) setWarnings((w) => [...w, "Failed to resolve payment link."]);
      })
      .finally(() => {
        if (!cancelled) setResolving(false);
      });
    return () => { cancelled = true; };
  }, [initialPrefill.ref]);

  const canContinue = useMemo(
    () =>
      Boolean(
        selectedWalletId &&
          (recipientType === "address" ? to.trim() : toEmailHash.trim())
      ),
    [recipientType, selectedWalletId, to, toEmailHash]
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
                recipientType,
                to: recipientType === "address" ? to.trim() : undefined,
                to_email_hash: recipientType === "email" ? toEmailHash : undefined,
                email_display: recipientType === "email" ? emailDisplay : undefined,
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

        {resolving ? (
          <div className="card card-pad" style={{ textAlign: "center" }}>
            <p className="text-muted">Resolving payment link…</p>
          </div>
        ) : null}

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

              {recipientType === "email" ? (
                <div className="field">
                  <label>Recipient email</label>
                  <input value={emailDisplay} readOnly />
                </div>
              ) : (
                <div className="field">
                  <label>Recipient address</label>
                  <input
                    value={to}
                    onChange={(event) => setTo(event.target.value)}
                    placeholder="0x\u2026"
                    style={{ fontFamily: "var(--font-mono)" }}
                  />
                </div>
              )}

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
                  className={`chip${token === "reur" ? " active" : ""}`}
                  onClick={() => setToken("reur")}
                >
                  rEUR
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
                disabled={!canContinue || resolving}
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
