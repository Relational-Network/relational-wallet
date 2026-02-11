// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { SignUp } from "@clerk/nextjs";

/**
 * Sign-up page using Clerk's hosted UI.
 */
export default function SignUpPage() {
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
      <SignUp />
    </main>
  );
}
