// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import type { ReactNode } from "react";

interface SimpleWalletShellProps {
  topBar?: ReactNode;
  children: ReactNode;
}

export function SimpleWalletShell({ topBar, children }: SimpleWalletShellProps) {
  return (
    <main className="app-container">
      {topBar && <header className="app-top-bar">{topBar}</header>}
      {children}
      <footer className="app-footer">        \u00a9 {new Date().getFullYear()} Relational Network \u00b7{" "}        <a href="https://github.com/Relational-Network/relational-wallet" target="_blank" rel="noopener noreferrer">
          Open source · AGPL-3.0
        </a>
      </footer>
    </main>
  );
}
