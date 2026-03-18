// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { auth } from "@clerk/nextjs/server";
import { redirect } from "next/navigation";
import { BootstrapConsole } from "./BootstrapConsole";

export default async function WalletBootstrapPage() {
  const { userId, sessionClaims } = await auth();
  if (!userId) {
    redirect("/sign-in");
  }

  const role = (sessionClaims?.publicMetadata as { role?: string } | undefined)?.role;
  if (role !== "admin") {
    redirect("/wallets");
  }

  return <BootstrapConsole />;
}

