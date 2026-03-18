// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";

/**
 * Wallet not found page.
 */
export default function WalletNotFound() {
  return (
    <SimpleWalletShell>
      <div className="card card-pad" style={{ textAlign: "center" }}>
        <h1 style={{ margin: 0, fontSize: "1.5rem" }}>Wallet not found</h1>
        <p className="text-secondary" style={{ margin: "0.75rem 0 0" }}>
          The wallet you are looking for does not exist or you do not have access to it.
        </p>
        <div style={{ marginTop: "1rem" }}>
          <Link href="/wallets" className="btn btn-primary">
            Back to wallets
          </Link>
        </div>
      </div>
    </SimpleWalletShell>
  );
}
