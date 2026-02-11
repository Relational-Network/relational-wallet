// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { SignIn } from "@clerk/nextjs";

/**
 * Sign-in page using Clerk's hosted UI.
 */
export default function SignInPage() {
  return (
    <main
      style={{
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        minHeight: "100vh",
        padding: "2rem",
      }}
    >
      <SignIn />
    </main>
  );
}
