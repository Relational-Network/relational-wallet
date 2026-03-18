// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { CopyAddress } from "@/components/CopyAddress";

interface PrimaryBalanceCardProps {
  walletLabel: string;
  walletAddress: string;
  avaxBalance: string;
  reurBalance: string;
  loading?: boolean;
  refreshing?: boolean;
}

/**
 * Primary wallet balance card.
 *
 * Layout: wallet name + address row, then AVAX and rEUR balance tiles.
 * Uses CSS blur for loading placeholders to avoid CLS.
 */
export function PrimaryBalanceCard({
  walletLabel,
  walletAddress,
  avaxBalance,
  reurBalance,
  loading = false,
  refreshing = false,
}: PrimaryBalanceCardProps) {
  const pending = loading || refreshing;

  return (
    <article className="balance-card" aria-busy={pending}>
      {/* ── Header: wallet name + address ──────────────────────── */}
      <div className="balance-card-header">
        <span className="balance-card-name">{walletLabel}</span>
        <CopyAddress address={walletAddress} showAddress />
      </div>

      {refreshing && (
        <span className="balance-card-status">Updating…</span>
      )}

      {/* ── Token balances ─────────────────────────────────────── */}
      <div className="balance-tokens">
        <div className="balance-token-tile">
          <div className="balance-token-label">AVAX</div>
          <div className={`balance-token-value${pending ? " loading-blur" : ""}`}>
            {avaxBalance || "0"}
          </div>
        </div>
        <div className="balance-token-tile">
          <div className="balance-token-label">rEUR</div>
          <div className={`balance-token-value${pending ? " loading-blur" : ""}`}>
            {reurBalance || "0"}
          </div>
        </div>
      </div>
    </article>
  );
}
