// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { auth, currentUser } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { apiClient, type UserMeResponse } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { SimpleWalletShell } from "@/components/SimpleWalletShell";
import { TokenDisplay } from "@/components/TokenDisplay";

export default async function AccountPage() {
  const { userId } = await auth();
  if (!userId) redirect("/sign-in");

  const clerkUser = await currentUser();
  const token = await getSessionToken();

  let backendUser: UserMeResponse | null = null;
  let backendError: string | null = null;

  if (token) {
    const response = await apiClient.getCurrentUser(token);
    if (response.success) {
      backendUser = response.data;
    } else if (response.error.status !== 401) {
      backendError = "Unable to fetch backend user info";
    }
  }

  return (
    <SimpleWalletShell
      topBar={
        <>
          <div className="app-top-left">
            <span style={{ fontWeight: 700 }}>Account</span>
          </div>
        </>
      }
    >
      <div className="stack">
        <TokenDisplay />

        <div className="card card-pad">
          <h3 className="section-title">Clerk profile</h3>
          <div className="grid-2" style={{ marginTop: "0.75rem" }}>
            <div>
              <div className="text-muted">User ID</div>
              <div style={{ fontFamily: "var(--font-mono)", fontSize: "0.8125rem" }}>{clerkUser?.id || "N/A"}</div>
            </div>
            <div>
              <div className="text-muted">Email</div>
              <div>{clerkUser?.emailAddresses?.[0]?.emailAddress || "N/A"}</div>
            </div>
            <div>
              <div className="text-muted">Name</div>
              <div>
                {clerkUser?.firstName && clerkUser?.lastName
                  ? `${clerkUser.firstName} ${clerkUser.lastName}`
                  : clerkUser?.firstName || "N/A"}
              </div>
            </div>
            <div>
              <div className="text-muted">Created</div>
              <div>{clerkUser?.createdAt ? new Date(clerkUser.createdAt).toLocaleString() : "N/A"}</div>
            </div>
          </div>
        </div>

        <div className="card card-pad">
          <h3 className="section-title">Developer tools</h3>
          {backendError ? (
            <div className="alert alert-error" style={{ marginTop: "0.75rem" }}>{backendError}</div>
          ) : backendUser ? (
            <div className="grid-2" style={{ marginTop: "0.75rem" }}>
              <div>
                <div className="text-muted">Backend User ID</div>
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "0.8125rem" }}>{backendUser.user_id}</div>
              </div>
              <div>
                <div className="text-muted">Role</div>
                <span className="badge badge-success" style={{ marginTop: "0.25rem" }}>
                  {backendUser.role}
                </span>
              </div>
              <div>
                <div className="text-muted">Session ID</div>
                <div style={{ fontFamily: "var(--font-mono)", fontSize: "0.8125rem" }}>{backendUser.session_id || "N/A"}</div>
              </div>
            </div>
          ) : (
            <p className="text-muted" style={{ marginTop: "0.75rem" }}>
              Backend info will appear once the enclave API is reachable.
            </p>
          )}
        </div>
      </div>
    </SimpleWalletShell>
  );
}
