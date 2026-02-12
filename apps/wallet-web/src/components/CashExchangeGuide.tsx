// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import Link from "next/link";

type CashGuideStep = "receive" | "send" | "history";

interface CashExchangeGuideProps {
  walletId: string;
  currentStep: CashGuideStep;
}

const STEPS: Array<{
  id: CashGuideStep;
  title: string;
  description: string;
  href: (walletId: string) => string;
  cta: string;
}> = [
  {
    id: "receive",
    title: "Step 1: Share Payment Request",
    description: "Show your receive address or generate a pull-request link/QR.",
    href: (walletId) => `/wallets/${encodeURIComponent(walletId)}/receive`,
    cta: "Open Receive",
  },
  {
    id: "send",
    title: "Step 2: Send Stable Value",
    description: "Sender fills recipient and amount, then confirms transfer.",
    href: (walletId) => `/wallets/${encodeURIComponent(walletId)}/send?flow=cash`,
    cta: "Open Send",
  },
  {
    id: "history",
    title: "Step 3: Verify Settlement",
    description: "Both parties confirm final status in transaction history.",
    href: (walletId) => `/wallets/${encodeURIComponent(walletId)}/transactions?flow=cash`,
    cta: "Open History",
  },
];

export function CashExchangeGuide({ walletId, currentStep }: CashExchangeGuideProps) {
  const currentIndex = STEPS.findIndex((step) => step.id === currentStep);

  return (
    <section
      style={{
        border: "1px solid #c7e6ff",
        borderRadius: "8px",
        padding: "1rem",
        marginBottom: "1.5rem",
        backgroundColor: "#f4fbff",
      }}
    >
      <h2 style={{ margin: "0 0 0.35rem 0", fontSize: "1.05rem", color: "#114a75" }}>
        Cash Exchange Guide
      </h2>
      <p style={{ margin: "0 0 0.85rem 0", color: "#326892", fontSize: "0.875rem" }}>
        Guided demo flow: receive request, send transfer, confirm on-chain record.
      </p>

      <div style={{ display: "grid", gap: "0.6rem" }}>
        {STEPS.map((step, index) => {
          const isCurrent = step.id === currentStep;
          const isDone = index < currentIndex;
          const statusLabel = isCurrent ? "Current" : isDone ? "Done" : "Next";

          return (
            <div
              key={step.id}
              style={{
                border: isCurrent ? "1px solid #4aa3e6" : "1px solid #d8e8f7",
                borderRadius: "6px",
                padding: "0.75rem",
                backgroundColor: isCurrent ? "#e8f4ff" : "#fff",
                display: "grid",
                gap: "0.35rem",
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  gap: "1rem",
                }}
              >
                <strong style={{ color: "#1c3f5f", fontSize: "0.95rem" }}>{step.title}</strong>
                <span
                  style={{
                    fontSize: "0.7rem",
                    fontWeight: "bold",
                    color: isCurrent ? "#0d5d9c" : isDone ? "#1d6a3f" : "#6b7785",
                    backgroundColor: isCurrent ? "#d1ebff" : isDone ? "#def5e8" : "#edf1f5",
                    padding: "0.15rem 0.45rem",
                    borderRadius: "999px",
                  }}
                >
                  {statusLabel}
                </span>
              </div>
              <p style={{ margin: 0, color: "#4f6a81", fontSize: "0.82rem" }}>{step.description}</p>
              <div>
                <Link
                  href={step.href(walletId)}
                  style={{
                    display: "inline-block",
                    marginTop: "0.1rem",
                    color: "#0b5ea6",
                    textDecoration: "none",
                    fontWeight: 600,
                    fontSize: "0.82rem",
                  }}
                >
                  {step.cta} →
                </Link>
              </div>
            </div>
          );
        })}
      </div>
    </section>
  );
}
