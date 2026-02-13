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
      <footer className="landing-footer">
        © {new Date().getFullYear()} Relational Network ·{" "}
        <a href="https://github.com/Relational-Network/relational-wallet" target="_blank" rel="noopener noreferrer">
          Source code (AGPL-3.0)
        </a>
      </footer>
    </main>
  );
}
