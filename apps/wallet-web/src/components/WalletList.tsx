// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import type { components } from "@/types/api";
import { WalletCard } from "./WalletCard";

type WalletResponse = components["schemas"]["WalletResponse"];

interface WalletListProps {
  wallets: WalletResponse[];
}

/**
 * List component displaying all user wallets.
 */
export function WalletList({ wallets }: WalletListProps) {
  if (wallets.length === 0) {
    return (
      <div style={{ textAlign: "center", padding: "2rem", color: "#666" }}>
        <p>No wallets found.</p>
        <p>
          <Link href="/wallets/new">Create your first wallet</Link>
        </p>
      </div>
    );
  }

  return (
    <div>
      {wallets.map((wallet) => (
        <Link
          key={wallet.wallet_id}
          href={`/wallets/${wallet.wallet_id}`}
          style={{ textDecoration: "none", color: "inherit", display: "block" }}
        >
          <WalletCard wallet={wallet} />
        </Link>
      ))}
    </div>
  );
}
