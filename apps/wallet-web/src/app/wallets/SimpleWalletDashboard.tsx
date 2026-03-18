// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { UserButton } from "@clerk/nextjs";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
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
import { PendingFiatRequests } from "@/components/PendingFiatRequests";
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
  | "create_wallet"
  | null;

const DEFAULT_PROVIDER = "truelayer_sandbox";
const REUR_FUJI_ADDRESS = "0x76568bed5acf1a5cd888773c8cae9ea2a9131a63";

function tokenLabel(token: string): string {
  if (token === "native") return "AVAX";
  const normalized = token.toLowerCase();
  if (normalized === REUR_FUJI_ADDRESS) return "rEUR";
  return "TOKEN";
}

function shortenAddress(address: string) {
  if (address.length < 18) return address;
  return `${address.slice(0, 9)}...${address.slice(-7)}`;
}

function mapActivity(response: TransactionListResponse): DashboardActivityItem[] {
  return response.transactions.slice(0, 5).map((transaction) => ({
    id: transaction.tx_hash,
    title: `${transaction.direction === "sent" ? "Sent" : "Received"} ${transaction.amount} ${tokenLabel(transaction.token)}`,
    subtitle: shortenAddress(transaction.tx_hash),
    status: transaction.status,
    timestamp: transaction.timestamp,
  }));
}

function CreateWalletDialog({
  open,
  onClose,
  onCreateWallet,
}: {
  open: boolean;
  onClose: () => void;
  onCreateWallet: (label?: string) => Promise<void>;
}) {
  const [label, setLabel] = useState("");
  const [creating, setCreating] = useState(false);
  const [dialogError, setDialogError] = useState<string | null>(null);

  const handleCreate = async () => {
    setCreating(true);
    setDialogError(null);
    try {
      await onCreateWallet(label.trim() || undefined);
      setLabel("");
      onClose();
    } catch (err) {
      setDialogError(err instanceof Error ? err.message : "Create failed");
    } finally {
      setCreating(false);
    }
  };

  return (
    <ActionDialog open={open} onClose={onClose} title="Create your first wallet">
      <div className="stack">
        <p className="text-muted" style={{ margin: 0, textAlign: "center" }}>
          You need a wallet to send, receive, and manage funds on Avalanche.
        </p>
        <div className="field">
          <label>Wallet label (optional)</label>
          <input
            className="input"
            value={label}
            onChange={(e) => setLabel(e.target.value)}
            placeholder="e.g. Personal, Savings"
            disabled={creating}
          />
        </div>
        {dialogError ? <div className="alert alert-error">{dialogError}</div> : null}
        <button
          type="button"
          className="btn btn-primary"
          onClick={() => void handleCreate()}
          disabled={creating}
        >
          {creating ? "Creating…" : "Create wallet"}
        </button>
      </div>
    </ActionDialog>
  );
}

function DashboardShellSkeleton() {
  return (
    <SimpleWalletShell>
      <div className="wallet-dashboard-shell wallet-dashboard-skeleton" aria-hidden="true">
        <div className="row-between">
          <div className="skeleton" style={{ width: "9.5rem", height: "2.5rem", borderRadius: "999px" }} />
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <div className="skeleton" style={{ width: "5.5rem", height: "2.25rem", borderRadius: "999px" }} />
            <div className="clerk-avatar-slot">
              <div className="skeleton" style={{ width: "100%", height: "100%", borderRadius: "999px" }} />
            </div>
          </div>
        </div>

        <article className="balance-card">
          <div className="balance-card-header">
            <div className="skeleton" style={{ width: "8rem", height: "1rem" }} />
            <div className="address-placeholder">0x00000000...000000</div>
          </div>
          <div className="balance-tokens">
            <div className="balance-token-tile">
              <div className="balance-token-label">AVAX</div>
              <div className="balance-token-value loading-blur">0.0000</div>
            </div>
            <div className="balance-token-tile">
              <div className="balance-token-label">rEUR</div>
              <div className="balance-token-value loading-blur">0.00</div>
            </div>
          </div>
        </article>

        <div className="quick-actions">
          {[1, 2, 3, 4].map((item) => (
            <div key={item} className="quick-action-btn wallet-skeleton-action">
              <span className="icon-circle">
                <div className="skeleton" style={{ width: "1rem", height: "1rem", borderRadius: "999px" }} />
              </span>
              <div className="skeleton" style={{ width: "3.5rem", height: "0.75rem" }} />
            </div>
          ))}
        </div>

        <div className="card card-pad">
          <div className="section-header" style={{ marginBottom: "0.5rem" }}>
            <div className="skeleton" style={{ width: "8rem", height: "1rem" }} />
            <div className="skeleton" style={{ width: "4.5rem", height: "2rem", borderRadius: "999px" }} />
          </div>
          {[1, 2, 3, 4, 5].map((item) => (
            <div key={item} className="activity-row" style={{ opacity: 0.45 }}>
              <div className="activity-icon" style={{ background: "var(--bg-subtle)" }} />
              <div className="activity-details">
                <div className="skeleton" style={{ width: item % 2 === 0 ? "58%" : "74%", height: "0.875rem", marginBottom: "0.25rem" }} />
                <div className="skeleton" style={{ width: "42%", height: "0.625rem" }} />
              </div>
              <div className="skeleton" style={{ width: "5rem", height: "1.5rem", borderRadius: "999px" }} />
            </div>
          ))}
        </div>
      </div>
    </SimpleWalletShell>
  );
}

interface SimpleWalletDashboardProps {
  initialWallets?: WalletResponse[];
  initialSelectedWalletId?: string | null;
  initialBalance?: BalanceResponse | null;
  initialTransactions?: TransactionListResponse | null;
}

export function SimpleWalletDashboard({
  initialWallets,
  initialSelectedWalletId,
  initialBalance,
  initialTransactions,
}: SimpleWalletDashboardProps = {}) {
  const hasSSRData = !!(initialWallets && initialWallets.length > 0);
  const ssrHasDetails = hasSSRData && !!(initialBalance || initialTransactions);

  const [wallets, setWallets] = useState<WalletResponse[]>(initialWallets ?? []);
  const [selectedWalletId, setSelectedWalletId] = useState<string | null>(initialSelectedWalletId ?? null);
  const [activeDialog, setActiveDialog] = useState<ActiveDialog>(null);
  const [loadingWallets, setLoadingWallets] = useState(!hasSSRData);
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [detailsLoaded, setDetailsLoaded] = useState(ssrHasDetails);
  const [error, setError] = useState<string | null>(null);
  const [balance, setBalance] = useState<BalanceResponse | null>(initialBalance ?? null);
  const [activity, setActivity] = useState<DashboardActivityItem[]>(
    initialTransactions ? mapActivity(initialTransactions) : []
  );
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
  const [bookmarksWalletId, setBookmarksWalletId] = useState<string | null>(null);
  const [txRefreshKey, setTxRefreshKey] = useState(0);

  const [provider, setProvider] = useState(DEFAULT_PROVIDER);
  const [onRampAmount, setOnRampAmount] = useState("25");
  const [onRampNote, setOnRampNote] = useState("");
  const [offRampAmount, setOffRampAmount] = useState("10");
  const [offRampNote, setOffRampNote] = useState("");
  const [offRampName, setOffRampName] = useState("");
  const [offRampIban, setOffRampIban] = useState("");
  const [fiatSubmitting, setFiatSubmitting] = useState<"on" | "off" | null>(null);
  const [fiatResult, setFiatResult] = useState<{
    requestId: string;
    actionUrl: string | null;
    status: FiatRequest["status"];
    serviceWalletAddress: string | null;
    failureReason: string | null;
  } | null>(null);
  const [pendingKey, setPendingKey] = useState(0);
  const [latestFiatRequest, setLatestFiatRequest] = useState<FiatRequest | null>(null);
  const providerPopupRef = useRef<Window | null>(null);
  const walletDetailsInFlightRef = useRef<Promise<void> | null>(null);
  const walletDetailsInFlightWalletRef = useRef<string | null>(null);
  const walletSnapshotRefreshTimersRef = useRef<ReturnType<typeof setTimeout>[]>([]);
  const walletSnapshotRefreshSequenceRef = useRef(0);

  const openProviderPopup = useCallback((url: string) => {
    const width = 500;
    const height = 700;
    const left = Math.round(window.screenX + (window.outerWidth - width) / 2);
    const top = Math.round(window.screenY + (window.outerHeight - height) / 2);
    const popup = window.open(
      url,
      "fiat-provider",
      `width=${width},height=${height},left=${left},top=${top},toolbar=no,menubar=no,location=yes`
    );
    if (popup) {
      providerPopupRef.current = popup;
    } else {
      // Popup blocked — fall back to new tab
      window.open(url, "_blank", "noopener,noreferrer");
    }
  }, []);

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

  const clearWalletSnapshotRefreshes = useCallback(() => {
    walletSnapshotRefreshSequenceRef.current += 1;
    for (const timer of walletSnapshotRefreshTimersRef.current) {
      clearTimeout(timer);
    }
    walletSnapshotRefreshTimersRef.current = [];
  }, []);

  const refreshWalletSnapshot = useCallback(async (
    walletId: string,
    options?: {
      silent?: boolean;
      syncTransactionHistory?: boolean;
    }
  ) => {
    const silent = options?.silent ?? false;
    const syncTransactionHistory = options?.syncTransactionHistory ?? false;

    if (
      walletDetailsInFlightRef.current &&
      walletDetailsInFlightWalletRef.current === walletId
    ) {
      await walletDetailsInFlightRef.current;
      if (syncTransactionHistory) {
        setTxRefreshKey((current) => current + 1);
      }
      return;
    }

    let currentFetch: Promise<void> | null = null;
    currentFetch = (async () => {
      if (!silent) setLoadingDetails(true);

      try {
        const [balanceResponse, txResponse] = await Promise.all([
          fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/balance?network=fuji`, {
            method: "GET",
            credentials: "include",
          }),
          fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}/transactions?network=fuji&limit=5`, {
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
          if (syncTransactionHistory) {
            setTxRefreshKey((current) => current + 1);
          }
        } else {
          setActivity([]);
          if (syncTransactionHistory && txResponse.status === 404) {
            setTxRefreshKey((current) => current + 1);
          }
        }
      } catch {
        // Non-critical: keep existing balance/activity or show zeros.
      } finally {
        if (!silent) setLoadingDetails(false);
        setDetailsLoaded(true);
      }
    })().finally(() => {
      if (walletDetailsInFlightRef.current === currentFetch) {
        walletDetailsInFlightRef.current = null;
        walletDetailsInFlightWalletRef.current = null;
      }
    });

    walletDetailsInFlightRef.current = currentFetch;
    walletDetailsInFlightWalletRef.current = walletId;
    await currentFetch;
  }, []);

  const invalidateWalletSnapshot = useCallback((
    walletId: string,
    options?: {
      followUp?: boolean;
      syncTransactionHistory?: boolean;
    }
  ) => {
    const followUp = options?.followUp ?? false;
    const syncTransactionHistory = options?.syncTransactionHistory ?? false;

    clearWalletSnapshotRefreshes();
    void refreshWalletSnapshot(walletId, {
      silent: true,
      syncTransactionHistory,
    });

    if (!followUp) {
      return;
    }

    const refreshSequence = walletSnapshotRefreshSequenceRef.current;
    for (const delayMs of [3_000, 8_000, 15_000]) {
      const timer = setTimeout(() => {
        if (walletSnapshotRefreshSequenceRef.current !== refreshSequence) {
          return;
        }

        void refreshWalletSnapshot(walletId, {
          silent: true,
          syncTransactionHistory,
        });
      }, delayMs);
      walletSnapshotRefreshTimersRef.current.push(timer);
    }
  }, [clearWalletSnapshotRefreshes, refreshWalletSnapshot]);

  // Listen for postMessage from callback popup
  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      if (event.origin !== window.location.origin) return;
      if (event.data?.type === "fiat-callback-complete") {
        providerPopupRef.current?.close();
        providerPopupRef.current = null;
        if (selectedWalletId) {
          invalidateWalletSnapshot(selectedWalletId, {
            followUp: true,
            syncTransactionHistory: true,
          });
        }
        setPendingKey((k) => k + 1);
        setActiveDialog(null);
        setFiatResult(null);
      }
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [invalidateWalletSnapshot, selectedWalletId]);

  // Skip initial wallet fetch if SSR already provided the wallet list.
  // Client-side actions (create/delete wallet) call fetchWallets() directly,
  // and the 60 s periodic poller handles background sync.
  useEffect(() => {
    if (hasSSRData) return;
    void fetchWallets();
  }, [fetchWallets, hasSSRData]);

  // Auto-open create wallet dialog when user has no wallets
  useEffect(() => {
    if (!loadingWallets && wallets.length === 0) {
      setActiveDialog("create_wallet");
    }
  }, [loadingWallets, wallets.length]);

  const fetchBookmarks = useCallback(async (walletId: string) => {
    try {
      const response = await fetch(
        `/api/proxy/v1/bookmarks?wallet_id=${encodeURIComponent(walletId)}`,
        { method: "GET", credentials: "include" }
      );
      if (response.ok) {
        const data: Bookmark[] = await response.json();
        setBookmarks(data);
        setBookmarksWalletId(walletId);
      }
    } catch {
      // Non-critical — keep empty bookmarks.
    }
  }, []);

  // Skip the initial balance/transaction fetch if SSR already provided data
  // for the selected wallet. Once the user switches to a different wallet
  // the ref is consumed so subsequent navigations always fetch fresh data.
  const ssrDetailsUsedRef = useRef(false);
  useEffect(() => {
    if (!selectedWalletId) return;
    if (
      !ssrDetailsUsedRef.current &&
      ssrHasDetails &&
      selectedWalletId === initialSelectedWalletId
    ) {
      ssrDetailsUsedRef.current = true;
      return;
    }
    ssrDetailsUsedRef.current = true;
    setDetailsLoaded(false);
    setBalance(null);
    setActivity([]);
    setBookmarks([]);
    setBookmarksWalletId(null);
    void refreshWalletSnapshot(selectedWalletId);
  }, [selectedWalletId, refreshWalletSnapshot, ssrHasDetails, initialSelectedWalletId]);

  // Periodic balance polling — keeps AVAX/rEUR in sync with on-chain state.
  // 60s interval is sufficient; balance changes are infrequent.
  useEffect(() => {
    if (!selectedWalletId) return;
    const interval = setInterval(() => {
      void refreshWalletSnapshot(selectedWalletId, { silent: true });
    }, 60_000);
    return () => clearInterval(interval);
  }, [selectedWalletId, refreshWalletSnapshot]);

  useEffect(() => {
    clearWalletSnapshotRefreshes();
    setLatestFiatRequest(null);
  }, [selectedWalletId, clearWalletSnapshotRefreshes]);

  useEffect(() => {
    if (activeDialog !== "on_ramp" && activeDialog !== "off_ramp") return;
    void fetchProviders();
  }, [activeDialog, fetchProviders]);

  const createWallet = async (label?: string) => {
    setError(null);

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
  };

  const deleteWallet = async (walletId: string) => {
    const response = await fetch(`/api/proxy/v1/wallets/${encodeURIComponent(walletId)}`, {
      method: "DELETE",
      credentials: "include",
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(text || `Delete failed (${response.status})`);
    }

    await fetchWallets();
  };

  const createOnRamp = async () => {
    if (!selectedWalletId) return;

    setFiatSubmitting("on");
    setError(null);
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
      setFiatResult({
        requestId: payload.request_id,
        actionUrl: payload.provider_action_url || null,
        status: payload.status,
        serviceWalletAddress: payload.service_wallet_address || null,
        failureReason: payload.failure_reason || null,
      });
      // Optimistically inject the new request into PendingFiatRequests
      setLatestFiatRequest(payload);
      setOnRampNote("");
      // Auto-open provider popup if URL is available
      if (payload.provider_action_url) {
        openProviderPopup(payload.provider_action_url);
      } else {
        invalidateWalletSnapshot(selectedWalletId, {
          followUp: true,
          syncTransactionHistory: true,
        });
      }
      setPendingKey((k) => k + 1);
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
      setFiatResult({
        requestId: payload.request_id,
        actionUrl: payload.provider_action_url || null,
        status: payload.status,
        serviceWalletAddress: payload.service_wallet_address || null,
        failureReason: payload.failure_reason || null,
      });
      // Optimistically inject the new request into PendingFiatRequests
      setLatestFiatRequest(payload);
      setOffRampNote("");
      setOffRampName("");
      setOffRampIban("");
      setPendingKey((k) => k + 1);
      invalidateWalletSnapshot(selectedWalletId, {
        followUp: true,
        syncTransactionHistory: true,
      });
    } catch (offRampError) {
      setError(offRampError instanceof Error ? offRampError.message : "Off-ramp request failed");
    } finally {
      setFiatSubmitting(null);
    }
  };

  const handleWalletSnapshotInvalidation = useCallback(() => {
    if (!selectedWalletId) return;
    invalidateWalletSnapshot(selectedWalletId, {
      followUp: true,
      syncTransactionHistory: true,
    });
  }, [invalidateWalletSnapshot, selectedWalletId]);

  useEffect(() => () => {
    clearWalletSnapshotRefreshes();
  }, [clearWalletSnapshotRefreshes]);

  const avaxBalance = balance?.native_balance.balance_formatted ?? "0";
  const reurBalance =
    balance?.token_balances.find((token) => token.symbol.toUpperCase() === "REUR")
      ?.balance_formatted ?? "0";
  const initialLoadComplete = !loadingWallets;
  const dashboardLoading = loadingWallets || (selectedWallet !== null && loadingDetails && !detailsLoaded);
  const walletLabel = selectedWallet?.label || "Wallet";
  const walletAddress = selectedWallet?.public_address || "0x0000000000000000000000000000000000000000";
  const actionsDisabled = !selectedWallet || selectedWallet.status !== "active" || loadingWallets;

  if (!initialLoadComplete) {
    return <DashboardShellSkeleton />;
  }

  return (
    <SimpleWalletShell>
      <div className="wallet-dashboard-shell">
        <div className="row-between">
          <button
            type="button"
            className="btn btn-manage"
            onClick={() => setActiveDialog("manage")}
          >
            Manage Wallet(s)
          </button>
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <Link
              href="/wallets/bootstrap"
              className="btn btn-ghost"
              title="Bootstrap reserve wallet and fiat setup"
            >
              Bootstrap
            </Link>
            {process.env.NODE_ENV === "development" && selectedWalletId ? (
              <a
                href={`/wallets/${selectedWalletId}`}
                style={{ fontSize: "0.6875rem", color: "var(--ink-muted)", textDecoration: "none", opacity: 0.6 }}
                title="Dev: view wallet detail page"
              >
                details →
              </a>
            ) : null}
            <div className="clerk-avatar-slot">
              <UserButton />
            </div>
          </div>
        </div>
        {error ? <div className="alert alert-error">{error}</div> : null}

        <PrimaryBalanceCard
          walletLabel={walletLabel}
          walletAddress={walletAddress}
          avaxBalance={avaxBalance}
          reurBalance={reurBalance}
          loading={dashboardLoading}
          refreshing={loadingDetails && detailsLoaded}
        />

        <PrimaryActions
          disabled={actionsDisabled}
          onSend={() => {
            if (selectedWalletId && bookmarksWalletId !== selectedWalletId) {
              void fetchBookmarks(selectedWalletId);
            }
            setActiveDialog("send");
          }}
          onReceive={() => setActiveDialog("receive")}
          onOnRamp={() => setActiveDialog("on_ramp")}
          onOffRamp={() => setActiveDialog("off_ramp")}
        />

        {selectedWallet && selectedWallet.status === "active" ? (
          <PendingFiatRequests
            walletId={selectedWallet.wallet_id}
            onProviderPopup={openProviderPopup}
            onInvalidateWalletSnapshot={handleWalletSnapshotInvalidation}
            refreshNonce={pendingKey}
            latestRequest={latestFiatRequest}
          />
        ) : null}

        <RecentActivityPreview
          items={activity}
          loading={dashboardLoading}
          onOpenAll={() => setActiveDialog("activity")}
        />
      </div>

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
              invalidateWalletSnapshot(selectedWallet.wallet_id, {
                syncTransactionHistory: true,
              });
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
          dialogClassName="activity-dialog"
          bodyClassName="activity-dialog-body"
        >
          <TransactionList walletId={selectedWallet.wallet_id} refreshKey={txRefreshKey} className="activity-transaction-list" />
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
          onDeleteWallet={async (walletId) => {
            await deleteWallet(walletId);
          }}
        />
      </ActionDialog>

      <CreateWalletDialog
        open={activeDialog === "create_wallet"}
        onClose={() => setActiveDialog(null)}
        onCreateWallet={createWallet}
      />

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
                <p className="fiat-result-detail">Status: {fiatResult.status}</p>
              </div>
              {fiatResult.failureReason ? (
                <div className="alert alert-error">{fiatResult.failureReason}</div>
              ) : null}
              {fiatResult.actionUrl ? (
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => openProviderPopup(fiatResult.actionUrl!)}
                >
                  Authorize with bank
                </button>
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
                <p className="fiat-result-detail">Status: {fiatResult.status}</p>
              </div>
              {fiatResult.serviceWalletAddress ? (
                <div className="alert success">
                  Send rEUR to reserve wallet: <span className="mono">{fiatResult.serviceWalletAddress}</span>
                </div>
              ) : null}
              {fiatResult.failureReason ? (
                <div className="alert alert-error">{fiatResult.failureReason}</div>
              ) : null}
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
