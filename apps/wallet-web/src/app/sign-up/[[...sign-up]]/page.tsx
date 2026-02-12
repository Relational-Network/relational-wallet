// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { SignUp } from "@clerk/nextjs";

/**
 * Clerk sign-up route with branded shell.
 */
export default function SignUpPage() {
  return (
    <main className="auth-layout">
      <section className="auth-shell">
        <article className="auth-copy">
          <span className="badge badge-brand">Create account</span>
          <h1>Set up your wallet in seconds.</h1>
          <p>Lightweight account creation for rapid onboarding.</p>
          <div style={{ marginTop: "1rem" }}>
            <Link className="btn btn-secondary" href="/">Back home</Link>
          </div>
        </article>
        <article className="auth-card">
          <SignUp />
        </article>
      </section>
    </main>
  );
}
