// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { SignIn } from "@clerk/nextjs";

/**
 * Clerk sign-in route with branded shell.
 */
export default function SignInPage() {
  return (
    <main className="auth-layout">
      <section className="auth-shell">
        <article className="auth-copy">
          <span className="badge badge-brand">Welcome back</span>
          <h1>Sign in to your wallet.</h1>
          <p>Continue with your credentials and jump straight into wallet operations.</p>
          <div style={{ marginTop: "1rem" }}>
            <Link className="btn btn-secondary" href="/">Back home</Link>
          </div>
        </article>
        <article className="auth-card">
          <SignIn />
        </article>
      </section>
    </main>
  );
}
