// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";
import { UserButton } from "@clerk/nextjs";
import { auth, currentUser } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { apiClient, type UserMeResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { TokenDisplay } from "@/components/TokenDisplay";

/**
 * Account page (authenticated).
 *
 * Displays the current user's information and role claims.
 */
export default async function AccountPage() {
  const { userId } = await auth();

  if (!userId) {
    redirect("/sign-in");
  }

  // Get Clerk user information
  const clerkUser = await currentUser();
  const token = await getSessionToken();

  // Fetch user info from backend API
  // TODO: Enable real fetch once enclave backend is available
  let backendUser: UserMeResponse | null = null;
  let backendError: string | null = null;

  if (token) {
    const response = await apiClient.getCurrentUser(token);
    if (response.success) {
      backendUser = response.data;
    } else if (response.error.status !== 401) {
      // Don't show error for 401 - just means backend isn't available
      backendError = "Unable to fetch backend user info";
    }
  }

  return (
    <main style={{ padding: "2rem", maxWidth: "800px", margin: "0 auto" }}>
      <header
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: "2rem",
        }}
      >
        <div>
          <Link href="/wallets" style={{ color: "#666", textDecoration: "none" }}>
            ← Back to Wallets
          </Link>
          <h1 style={{ marginTop: "0.5rem" }}>Account</h1>
        </div>
        <UserButton />
      </header>

      {/* JWT Token Display for Testing */}
      <TokenDisplay />

      <section
        style={{
          border: "1px solid #ddd",
          borderRadius: "4px",
          padding: "1.5rem",
          marginBottom: "2rem",
        }}
      >
        <h2 style={{ marginTop: 0 }}>Clerk User Information</h2>

        <dl style={{ margin: 0 }}>
          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
            User ID
          </dt>
          <dd style={{ margin: "0.25rem 0 0 0", fontFamily: "monospace", color: "#333" }}>
            {clerkUser?.id || "N/A"}
          </dd>

          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
            Email
          </dt>
          <dd style={{ margin: "0.25rem 0 0 0", color: "#333" }}>
            {clerkUser?.emailAddresses?.[0]?.emailAddress || "N/A"}
          </dd>

          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
            Name
          </dt>
          <dd style={{ margin: "0.25rem 0 0 0", color: "#333" }}>
            {clerkUser?.firstName && clerkUser?.lastName
              ? `${clerkUser.firstName} ${clerkUser.lastName}`
              : clerkUser?.firstName || "N/A"}
          </dd>

          <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
            Created
          </dt>
          <dd style={{ margin: "0.25rem 0 0 0", color: "#333" }}>
            {clerkUser?.createdAt
              ? new Date(clerkUser.createdAt).toLocaleString()
              : "N/A"}
          </dd>
        </dl>
      </section>

      <section
        style={{
          border: "1px solid #ddd",
          borderRadius: "4px",
          padding: "1.5rem",
        }}
      >
        <h2 style={{ marginTop: 0 }}>Backend User Information</h2>

        {backendError ? (
          <p style={{ color: "#c00" }}>{backendError}</p>
        ) : backendUser ? (
          <dl style={{ margin: 0 }}>
            <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
              Backend User ID
            </dt>
            <dd style={{ margin: "0.25rem 0 0 0", fontFamily: "monospace", color: "#333" }}>
              {backendUser.user_id}
            </dd>

            <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
              Role
            </dt>
            <dd
              style={{
                margin: "0.25rem 0 0 0",
                display: "inline-block",
                padding: "0.25rem 0.5rem",
                backgroundColor: "#e0e0e0",
                borderRadius: "4px",
                fontFamily: "monospace",
                color: "#333",
              }}
            >
              {backendUser.role}
            </dd>

            {backendUser.session_id && (
              <>
                <dt style={{ fontWeight: "bold", color: "#666", marginTop: "1rem" }}>
                  Session ID
                </dt>
                <dd style={{ margin: "0.25rem 0 0 0", fontFamily: "monospace", color: "#333" }}>
                  {backendUser.session_id}
                </dd>
              </>
            )}
          </dl>
        ) : (
          <p style={{ color: "#666" }}>
            Backend user information will be available once the enclave backend is connected.
          </p>
        )}
      </section>

      {/* TODO: Add admin dashboard link for admin users */}
    </main>
  );
}
