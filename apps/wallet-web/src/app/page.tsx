// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { SignedOut } from "@clerk/nextjs";
import { auth } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { Shield, Zap, Wallet } from "lucide-react";

/**
 * Public landing page — fintech-style hero with feature cards.
 */
export default async function HomePage() {
  const { userId } = await auth();
  if (userId) {
    redirect("/wallets");
  }

  return (
    <div className="landing-root">
      {/* ── Nav ─────────────────────────────────────────────── */}
      <nav className="landing-nav">
        <div className="row" style={{ gap: "0.5rem" }}>
          <Shield size={22} strokeWidth={2.5} color="var(--brand)" />
          <span style={{ fontWeight: 800, fontSize: "1.0625rem" }}>
            Relational Wallet
          </span>
        </div>
        <SignedOut>
          <Link className="btn btn-ghost" href="/sign-in">
            Sign in
          </Link>
        </SignedOut>
      </nav>

      {/* ── Hero ────────────────────────────────────────────── */}
      <section className="landing-hero">
        <span className="badge badge-brand" style={{ marginBottom: "0.75rem" }}>
          Avalanche Fuji Testnet
        </span>
        <h1>
          Your money, protected by&nbsp;
          <span>hardware&nbsp;enclaves</span>
        </h1>
        <p>
          Send and receive USDC and AVAX with a wallet whose private keys never
          leave a secure Intel SGX enclave. Fast, simple, auditable.
        </p>

        <SignedOut>
          <div
            className="row"
            style={{
              justifyContent: "center",
              gap: "0.75rem",
              marginTop: "1.5rem",
            }}
          >
            <Link className="btn btn-primary" href="/sign-up">
              Get started — it&apos;s free
            </Link>
            <Link className="btn btn-secondary" href="/sign-in">
              Sign in
            </Link>
          </div>
        </SignedOut>
      </section>

      {/* ── Features ────────────────────────────────────────── */}
      <section className="landing-features">
        <article className="card card-pad landing-feature-card">
          <Shield size={28} color="var(--brand)" />
          <h3>Hardware-grade security</h3>
          <p>
            Private keys are generated and stored inside an Intel SGX enclave
            with remote attestation — they never leave the secure boundary.
          </p>
        </article>
        <article className="card card-pad landing-feature-card">
          <Zap size={28} color="var(--brand)" />
          <h3>Instant transactions</h3>
          <p>
            Send USDC or AVAX in seconds. Gas estimation, one-tap confirmation,
            and real-time status polling built in.
          </p>
        </article>
        <article className="card card-pad landing-feature-card">
          <Wallet size={28} color="var(--brand)" />
          <h3>Simple by design</h3>
          <p>
            No seed phrases to manage, no browser extensions required. Sign in,
            create a wallet, and start transacting.
          </p>
        </article>
      </section>

      {/* ── Footer ──────────────────────────────────────────── */}
      <footer className="landing-footer">
        © {new Date().getFullYear()} Relational Network ·{" "}
        <a href="https://github.com/Relational-Network/relational-wallet" target="_blank" rel="noopener noreferrer">
          Source code (AGPL-3.0)
        </a>
      </footer>
    </div>
  );
}
