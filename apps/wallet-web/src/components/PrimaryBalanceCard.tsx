// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { CopyAddress } from "@/components/CopyAddress";

interface PrimaryBalanceCardProps {
  walletLabel: string;
  walletAddress: string;
  avaxBalance: string;
  usdcBalance: string;
  reurBalance: string;
  loading?: boolean;
  refreshing?: boolean;
}

export function PrimaryBalanceCard({
  walletLabel,
  walletAddress,
  avaxBalance,
  usdcBalance,
  reurBalance,
  loading = false,
  refreshing = false,
}: PrimaryBalanceCardProps) {
  // Show $0.00 immediately — don't block with skeleton for blank wallets
  const primaryAmount = loading ? "$0.00" : `EUR ${reurBalance}`;
  const dimmed = loading || refreshing;

  return (
    <article className="balance-card">
      <div className="balance-card-label">
        {walletLabel}
        {(loading || refreshing) ? (
          <span style={{ marginLeft: "0.5rem", fontSize: "0.6875rem", opacity: 0.6 }}>
            {loading ? "Loading…" : "Updating…"}
          </span>
        ) : null}
      </div>
      <div className="balance-card-amount" style={dimmed ? { opacity: 0.5 } : undefined}>{primaryAmount}</div>
      <div className="balance-card-address">
        <CopyAddress address={walletAddress} showAddress />
      </div>

      <div className="balance-tokens">
        <div className="balance-token-tile">
          <div className="balance-token-label">AVAX</div>
          <div className="balance-token-value" style={dimmed ? { opacity: 0.5 } : undefined}>
            {loading ? "0" : avaxBalance}
          </div>
        </div>
        <div className="balance-token-tile">
          <div className="balance-token-label">USDC</div>
          <div className="balance-token-value" style={dimmed ? { opacity: 0.5 } : undefined}>
            {loading ? "0" : usdcBalance}
          </div>
        </div>
        <div className="balance-token-tile">
          <div className="balance-token-label">rEUR</div>
          <div className="balance-token-value" style={dimmed ? { opacity: 0.5 } : undefined}>
            {loading ? "0" : reurBalance}
          </div>
        </div>
      </div>
    </article>
  );
}
