// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { auth } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { apiClient } from "@/lib/api";
import { getSessionToken } from "@/lib/auth";
import { parsePaymentRequestQuery } from "@/lib/paymentRequest";
import { PayEntry } from "./PayEntry";

interface PayPageProps {
  searchParams: Promise<{
    to?: string;
    amount?: string;
    token?: string;
    note?: string;
  }>;
}

export default async function PayPage({ searchParams }: PayPageProps) {
  const { userId } = await auth();
  if (!userId) {
    redirect("/sign-in");
  }

  const token = await getSessionToken();
  if (!token) {
    redirect("/sign-in");
  }

  const response = await apiClient.listWallets(token);
  if (!response.success) {
    if (response.error.status === 401) {
      redirect("/sign-in");
    }
    return (
      <main className="pay-layout">
        <div className="pay-container">
          <div className="alert alert-error">Unable to load wallets: {response.error.message}</div>
        </div>
      </main>
    );
  }

  const activeWallets = response.data.wallets.filter((wallet) => wallet.status === "active");
  const query = await searchParams;
  const parsed = parsePaymentRequestQuery(query);

  return <PayEntry wallets={activeWallets} prefill={parsed.prefill} warnings={parsed.warnings} />;
}
