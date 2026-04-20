// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { auth } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { AdminPanel } from "./AdminPanel";

export default async function AdminPage() {
  const { userId, sessionClaims } = await auth();
  if (!userId) redirect("/sign-in");

  const isAdmin =
    (sessionClaims?.publicMetadata as { role?: string } | undefined)?.role ===
    "admin";

  if (!isAdmin) redirect("/wallets");

  return <AdminPanel />;
}
