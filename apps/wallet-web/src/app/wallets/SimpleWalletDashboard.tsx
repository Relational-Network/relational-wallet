// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { UserButton } from "@clerk/nextjs";
import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  Bookmark,
  FiatProviderListResponse,
  FiatRequest,
  TransactionListResponse,
  WalletListResponse,
  WalletResponse,
} from "@/lib/api";
import { AddressQRCode } from "@/components/AddressQRCode";
import { CopyAddress } from "@/components/CopyAddress";
import { ActionDialog } from "@/components/ActionDialog";
import { ManageWalletsSheet } from "@/components/ManageWalletsSheet";
import { PaymentRequestBuilder } from "@/app/wallets/[wallet_id]/receive/PaymentRequestBuilder";
import { SendForm } from "@/app/wallets/[wallet_id]/send/SendForm";
import { TransactionList } from "@/app/wallets/[wallet_id]/transactions/TransactionList";
import { PrimaryActions } from "@/components/PrimaryActions";
import { PrimaryBalanceCard } from "@/components/PrimaryBalanceCard";
import { RecentActivityPreview, type DashboardActivityItem } from "@/components/RecentActivityPreview";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";

interface TokenBalance {
  symbol: string;
  balance_formatted: string;
}

interface BalanceResponse {
  native_balance: TokenBalance;
  token_balances: TokenBalance[];
}

type ActiveDialog =
  | "send"
  | "receive"
  | "on_ramp"
  | "off_ramp"
  | "activity"
  | "manage"
  | null;

const DEFAULT_PROVIDER = "truelayer_sandbox";

function shortenAddress(address: string) {
  if (address.length < 18) return address;
  return `${address.slice(0, 9)}...${address.slice(-7)}`;
}

function mapActivity(response: TransactionListResponse): DashboardActivityItem[] {
  return response.transactions.slice(0, 5).map((transaction) => ({
    id: transaction.tx_hash,
    title: `${transaction.direction === "sent" ? "Sent" : "Received"} ${transaction.amount} ${transaction.token === "native" ? "AVAX" : "USDC"}`,
    subtitle: `${shortenAddress(transaction.tx_hash)} • ${new Date(transaction.timestamp).toLocaleString()}`,
    status: transaction.status,
    timestamp: transaction.timestamp,
  }));
}

export function SimpleWalletDashboard() {
  const [wallets, setWallets] = useState<WalletResponse[]>([]);
  const [selectedWalletId, setSelectedWalletId] = useState<string | null>(null);
  const [activeDialog, setActiveDialog] = useState<ActiveDialog>(null);
  const [loadingWallets, setLoadingWallets] = useState(true);
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [detailsLoaded, setDetailsLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [balance, setBalance] = useState<BalanceResponse | null>(null);
  const [activity, setActivity] = useState<DashboardActivityItem[]>([]);
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);

  const [provider, setProvider] = useState(DEFAULT_PROVIDER);
  const [onRampAmount, setOnRampAmount] = useState("25");
  const [onRampNote, setOnRampNote] = useState("");
  const [offRampAmount, setOffRampAmount] = useState("10");
  const [offRampNote, setOffRampNote] = useState("");
  const [offRampName, setOffRampName] = useState("");
  const [offRampIban, setOffRampIban] = useState("");
  const [fiatSubmitting, setFiatSubmitting] = useState<"on" | "off" | null>(null);
  const [fiatResult, setFiatResult] = useState<{ requestId: string; actionUrl: string | null } | null>(null);

  const selectedWallet = useMemo(
    () => wallets.find((wallet) => wallet.wallet_id === selectedWalletId) ?? null,
    [wallets, selectedWalletId]
  );

  const fetchWallets = useCallback(async () => {
    setLoadingWallets(true);
    setError(null);

    try {
      console.debug("[simple-hub] fetching wallets");
      const response = await fetch("/api/proxy/v1/wallets", {
        method: "GET",
        credentials: "include",
      });

      if (!response.ok) {
        const text = await response.text();
        setError(text || `Unable to load wallets (${response.status})`);
        return;
      }

      const payload: WalletListResponse = await response.json();
      const nextWallets = payload.wallets;
      setWallets(nextWallets);

      if (nextWallets.length === 0) {
        setSelectedWalletId(null);
        return;
      }

      setSelectedWalletId((currentId) => {
        if (currentId && nextWallets.some((wallet) => wallet.wallet_id === currentId)) {
          return currentId;
        }
        const firstActive = nextWallets.find((wallet) => wallet.status === "active");
        return firstActive?.wallet_id ?? nextWallets[0].wallet_id;
      });
    } catch (walletError) {
      setError(walletError instanceof Error ? walletError.message : "Wallet load failed");
    } finally {
      setLoadingWallets(false);
    }
  }, []);

  const fetchProviders = useCallback(async () => {
    try {
      const response = await fetch("/api/proxy/v1/fiat/providers", {
        method: "GET",
        credentials: "include",
      });

      if (!response.ok) return;

      const payload: FiatProviderListResponse = await response.json();
      if (!payload.providers.length) return;

      setProvider(payload.default_provider || payload.providers[0].provider_id);
    } catch {
      // Keep fallback provider.
    }
  }, []);

  const fetchWalletDetails = useCallback(async (walletId: string) => {
    setLoadingDetails(true);

    try {
      const [balanceResponse, txResponse] = await Promise.all([
        fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/balance?network=fuji`, {
          method: "GET",
          credentials: "include",
        }),
        fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/transactions?network=fuji`, {
          method: "GET",
          credentials: "include",
        }),
      ]);

      if (balanceResponse.ok) {
        const nextBalance: BalanceResponse = await balanceResponse.json();
        setBalance(nextBalance);
      } else {
        setBalance(null);
      }

      if (txResponse.ok) {
        const nextTransactions: TransactionListResponse = await txResponse.json();
        setActivity(mapActivity(nextTransactions));
      } else {
        setActivity([]);
      }
    } catch {
      // Non-critical: keep existing balance/activity or show zeros.
    } finally {
      setLoadingDetails(false);
      setDetailsLoaded(true);
    }
  }, []);

  useEffect(() => {
    void fetchWallets();
    void fetchProviders();
  }, [fetchWallets, fetchProviders]);

  const fetchBookmarks = useCallback(async (walletId: string) => {
    try {
      const response = await fetch(
        `/api/proxy/v1/bookmarks?wallet_id=${encodeURIComponent(walletId)}`,
        { method: "GET", credentials: "include" }
      );
      if (response.ok) {
        const data: Bookmark[] = await response.json();
        setBookmarks(data);
      }
    } catch {
      // Non-critical — keep empty bookmarks.
    }
  }, []);

  useEffect(() => {
    if (!selectedWalletId) return;
    setDetailsLoaded(false);
    setBalance(null);
    setActivity([]);
    void fetchWalletDetails(selectedWalletId);
    void fetchBookmarks(selectedWalletId);
  }, [selectedWalletId, fetchWalletDetails, fetchBookmarks]);

  const createWallet = async (label?: string) => {
    setError(null);
    setSuccess(null);

    const response = await fetch("/api/proxy/v1/wallets", {
      method: "POST",
      credentials: "include",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ label }),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(text || `Create wallet failed (${response.status})`);
    }

    const payload = await response.json();
    const walletId = payload.wallet?.wallet_id as string | undefined;

    await fetchWallets();
    if (walletId) {
      setSelectedWalletId(walletId);
    }

    setSuccess("Wallet created.");
  };

  const createOnRamp = async () => {
    if (!selectedWalletId) return;

    setFiatSubmitting("on");
    setError(null);
    setSuccess(null);
    setFiatResult(null);

    try {
      const response = await fetch("/api/proxy/v1/fiat/onramp/requests", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wallet_id: selectedWalletId,
          amount_eur: onRampAmount,
          provider,
          note: onRampNote.trim() || undefined,
        }),
      });

      if (!response.ok) {
        const text = await response.text();
        setError(text || `On-ramp failed (${response.status})`);
        return;
      }

      const payload = (await response.json()) as FiatRequest;
      setFiatResult({ requestId: payload.request_id, actionUrl: payload.provider_action_url || null });
      setOnRampNote("");
    } catch (onRampError) {
      setError(onRampError instanceof Error ? onRampError.message : "On-ramp request failed");
    } finally {
      setFiatSubmitting(null);
    }
  };

  const createOffRamp = async () => {
    if (!selectedWalletId) return;

    setFiatSubmitting("off");
    setError(null);
    setSuccess(null);
    setFiatResult(null);

    try {
      const response = await fetch("/api/proxy/v1/fiat/offramp/requests", {
        method: "POST",
        credentials: "include",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          wallet_id: selectedWalletId,
          amount_eur: offRampAmount,
          provider,
          note: offRampNote.trim() || undefined,
          beneficiary_account_holder_name: offRampName,
          beneficiary_iban: offRampIban,
        }),
      });

      if (!response.ok) {
        const text = await response.text();
        setError(text || `Off-ramp failed (${response.status})`);
        return;
      }

      const payload = (await response.json()) as FiatRequest;
      setFiatResult({ requestId: payload.request_id, actionUrl: null });
      setOffRampNote("");
      setOffRampName("");
      setOffRampIban("");
    } catch (offRampError) {
      setError(offRampError instanceof Error ? offRampError.message : "Off-ramp request failed");
    } finally {
      setFiatSubmitting(null);
    }
  };

  const avaxBalance = balance?.native_balance.balance_formatted ?? "0";
  const usdcBalance =
    balance?.token_balances.find((token) => token.symbol.toUpperCase() === "USDC")
      ?.balance_formatted ?? "0";

  return (
    <SimpleWalletShell>
      <div className="row-between">
        <button
          type="button"
          className="btn btn-ghost"
          onClick={() => setActiveDialog("manage")}
          style={{ fontWeight: 700, fontSize: "0.9375rem" }}
        >
          {selectedWallet?.label || "Wallet"} &#9662;
        </button>
        <UserButton />
      </div>
      {error ? <div className="alert alert-error">{error}</div> : null}
      {success ? <div className="alert alert-success">{success}</div> : null}

      {!selectedWallet ? (
        <div className="card card-pad empty-state">
          <h3>No wallet yet</h3>
          <p>Create your first wallet to unlock send, receive, and fiat actions.</p>
          <button
            type="button"
            className="btn btn-primary"
            style={{ marginTop: "0.75rem" }}
            onClick={() => setActiveDialog("manage")}
          >
            Create wallet
          </button>
        </div>
      ) : (
        <>
          <PrimaryBalanceCard
            walletLabel={selectedWallet.label || "Wallet"}
            walletAddress={selectedWallet.public_address}
            avaxBalance={avaxBalance}
            usdcBalance={usdcBalance}
            loading={loadingDetails && !detailsLoaded}
            refreshing={loadingDetails && detailsLoaded}
          />

          <PrimaryActions
            disabled={!selectedWallet || selectedWallet.status !== "active"}
            onSend={() => setActiveDialog("send")}
            onReceive={() => setActiveDialog("receive")}
            onOnRamp={() => setActiveDialog("on_ramp")}
            onOffRamp={() => setActiveDialog("off_ramp")}
          />

          <RecentActivityPreview
            items={activity}
            loading={loadingDetails && !detailsLoaded}
            onOpenAll={() => setActiveDialog("activity")}
          />
        </>
      )}

      {selectedWallet ? (
        <ActionDialog
          open={activeDialog === "send"}
          onClose={() => setActiveDialog(null)}
          title="Send"
          wide
        >
          <SendForm
            walletId={selectedWallet.wallet_id}
            publicAddress={selectedWallet.public_address}
            walletLabel={selectedWallet.label ?? null}
            shortcuts={bookmarks.map((b) => ({ id: b.id, name: b.name, address: b.address }))}
            mode="dialog"
            onRequestClose={() => setActiveDialog(null)}
            onComplete={() => {
              void fetchWalletDetails(selectedWallet.wallet_id);
            }}
          />
        </ActionDialog>
      ) : null}

      {selectedWallet ? (
        <ActionDialog
          open={activeDialog === "receive"}
          onClose={() => setActiveDialog(null)}
          title="Receive"
        >
          <div className="stack" style={{ alignItems: "center" }}>
            <p className="text-muted" style={{ margin: 0, textAlign: "center" }}>
              Scan or share your address to receive funds on Avalanche C-Chain.
            </p>
            <AddressQRCode address={selectedWallet.public_address} size={180} />
            <CopyAddress address={selectedWallet.public_address} />
            <hr className="divider" style={{ width: "100%" }} />
            <PaymentRequestBuilder recipientAddress={selectedWallet.public_address} compact />
          </div>
        </ActionDialog>
      ) : null}

      {selectedWallet ? (
        <ActionDialog
          open={activeDialog === "activity"}
          onClose={() => setActiveDialog(null)}
          title="All Activity"
          wide
        >
          <TransactionList walletId={selectedWallet.wallet_id} />
        </ActionDialog>
      ) : null}

      <ActionDialog
        open={activeDialog === "manage"}
        onClose={() => setActiveDialog(null)}
        title="Manage wallets"
      >
        <ManageWalletsSheet
          wallets={wallets}
          selectedWalletId={selectedWalletId}
          onSelectWallet={(walletId) => {
            setSelectedWalletId(walletId);
            setActiveDialog(null);
          }}
          onCreateWallet={async (label) => {
            await createWallet(label);
          }}
        />
      </ActionDialog>

      {selectedWallet ? (
        <ActionDialog
          open={activeDialog === "on_ramp"}
          onClose={() => { setActiveDialog(null); setFiatResult(null); }}
          title="On-Ramp"
        >
          {fiatResult && activeDialog === "on_ramp" ? (
            <div className="stack">
              <div className="fiat-result">
                <p className="fiat-result-title">✓ On-ramp request created</p>
                <p className="fiat-result-detail">Request ID: {fiatResult.requestId}</p>
              </div>
              {fiatResult.actionUrl ? (
                <a href={fiatResult.actionUrl} target="_blank" rel="noopener noreferrer" className="btn btn-primary">
                  Continue with provider →
                </a>
              ) : null}
              <button type="button" className="btn btn-secondary" onClick={() => { setActiveDialog(null); setFiatResult(null); }}>
                Done
              </button>
            </div>
          ) : (
            <div className="stack">
              <div className="field">
                <label>Amount (EUR)</label>
                <input value={onRampAmount} onChange={(event) => setOnRampAmount(event.target.value)} inputMode="decimal" />
              </div>
              <div className="field">
                <label>Note (optional)</label>
                <input value={onRampNote} onChange={(event) => setOnRampNote(event.target.value)} placeholder="Reference" />
              </div>
              {error && activeDialog === "on_ramp" ? <div className="alert alert-error">{error}</div> : null}
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void createOnRamp()}
                disabled={fiatSubmitting !== null || !onRampAmount.trim()}
              >
                {fiatSubmitting === "on" ? "Creating…" : "Create On-Ramp Request"}
              </button>
            </div>
          )}
        </ActionDialog>
      ) : null}

      {selectedWallet ? (
        <ActionDialog
          open={activeDialog === "off_ramp"}
          onClose={() => { setActiveDialog(null); setFiatResult(null); }}
          title="Off-Ramp"
        >
          {fiatResult && activeDialog === "off_ramp" ? (
            <div className="stack">
              <div className="fiat-result">
                <p className="fiat-result-title">✓ Off-ramp request created</p>
                <p className="fiat-result-detail">Request ID: {fiatResult.requestId}</p>
              </div>
              <button type="button" className="btn btn-secondary" onClick={() => { setActiveDialog(null); setFiatResult(null); }}>
                Done
              </button>
            </div>
          ) : (
            <div className="stack">
              <div className="field">
                <label>Amount (EUR)</label>
                <input value={offRampAmount} onChange={(event) => setOffRampAmount(event.target.value)} inputMode="decimal" />
              </div>
              <div className="field">
                <label>Account holder name</label>
                <input value={offRampName} onChange={(event) => setOffRampName(event.target.value)} />
              </div>
              <div className="field">
                <label>IBAN</label>
                <input value={offRampIban} onChange={(event) => setOffRampIban(event.target.value)} placeholder="DE89…" style={{ fontFamily: "var(--font-mono)" }} />
              </div>
              <div className="field">
                <label>Note (optional)</label>
                <input value={offRampNote} onChange={(event) => setOffRampNote(event.target.value)} placeholder="Reference" />
              </div>
              {error && activeDialog === "off_ramp" ? <div className="alert alert-error">{error}</div> : null}
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => void createOffRamp()}
                disabled={fiatSubmitting !== null || !offRampAmount.trim()}
              >
                {fiatSubmitting === "off" ? "Creating…" : "Create Off-Ramp Request"}
              </button>
            </div>
          )}
        </ActionDialog>
      ) : null}
    </SimpleWalletShell>
  );
}
