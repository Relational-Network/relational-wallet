// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import type { WalletResponse } from "@/lib/api";

interface WalletSwitcherProps {
  wallets: WalletResponse[];
  selectedWalletId: string | null;
  onSelect: (walletId: string) => void;
  loading?: boolean;
}

export function WalletSwitcher({ wallets, selectedWalletId, onSelect, loading = false }: WalletSwitcherProps) {
  return (
    <div className="wallet-switcher">
      <select
        value={selectedWalletId ?? ""}
        onChange={(event) => onSelect(event.target.value)}
        disabled={loading || wallets.length === 0}
      >
        {wallets.length === 0 ? <option value="">No wallets</option> : null}
        {wallets.map((wallet) => (
          <option key={wallet.wallet_id} value={wallet.wallet_id}>
            {wallet.label || "Wallet"} ({wallet.wallet_id.slice(0, 6)}…)
          </option>
        ))}
      </select>
    </div>
  );
}
