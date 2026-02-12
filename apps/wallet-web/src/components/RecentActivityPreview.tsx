// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

import { ArrowUpRight, ArrowDownLeft, Clock } from "lucide-react";

interface DashboardActivityItem {
  id: string;
  title: string;
  subtitle: string;
  status: "pending" | "confirmed" | "failed";
  timestamp: string;
  /** "sent" or "received" — parsed from title */
  direction?: "sent" | "received";
}

interface RecentActivityPreviewProps {
  items: DashboardActivityItem[];
  loading?: boolean;
  onOpenAll: () => void;
}

function statusLabel(status: DashboardActivityItem["status"]) {
  if (status === "confirmed") return "Confirmed";
  if (status === "failed") return "Failed";
  return "Pending";
}

function inferDirection(title: string): "sent" | "received" {
  return title.toLowerCase().startsWith("sent") ? "sent" : "received";
}

export function RecentActivityPreview({ items, loading = false, onOpenAll }: RecentActivityPreviewProps) {
  return (
    <div className="card card-pad">
      <div className="section-header" style={{ marginBottom: "0.5rem" }}>
        <h3 className="section-title">Recent activity</h3>
        {items.length > 0 ? (
          <button type="button" className="btn btn-ghost" onClick={onOpenAll}>
            View all
          </button>
        ) : null}
      </div>

      {loading ? (
        <div className="stack-sm">
          <div className="skeleton" style={{ width: "100%", height: "2.5rem" }} />
          <div className="skeleton" style={{ width: "80%", height: "2.5rem" }} />
        </div>
      ) : items.length > 0 ? (
        <div>
          {items.map((item) => {
            const dir = item.direction ?? inferDirection(item.title);
            return (
              <div key={item.id} className="activity-row">
                <div className={`activity-icon ${dir}`}>
                  {dir === "sent" ? <ArrowUpRight size={18} /> : <ArrowDownLeft size={18} />}
                </div>
                <div className="activity-details">
                  <div className="activity-title">{item.title}</div>
                  <div className="activity-subtitle">{item.subtitle}</div>
                </div>
                <div>
                  <span className={`status-chip ${item.status}`}>{statusLabel(item.status)}</span>
                </div>
              </div>
            );
          })}
        </div>
      ) : (
        <div className="empty-state" style={{ padding: "1.25rem 0.5rem" }}>
          <div className="empty-state-icon">
            <Clock size={22} />
          </div>
          <h3>No transactions yet</h3>
          <p>Send or receive funds to see your activity here. Use the faucet links below to get testnet tokens.</p>
          <div className="row" style={{ justifyContent: "center", marginTop: "0.75rem", gap: "0.5rem", flexWrap: "wrap" }}>
            <a href="https://core.app/tools/testnet-faucet/?subnet=c&token=c" target="_blank" rel="noopener noreferrer" className="btn btn-secondary" style={{ fontSize: "0.75rem" }}>
              AVAX Faucet
            </a>
            <a href="https://faucet.circle.com/" target="_blank" rel="noopener noreferrer" className="btn btn-secondary" style={{ fontSize: "0.75rem" }}>
              USDC Faucet
            </a>
          </div>
        </div>
      )}
    </div>
  );
}

export type { DashboardActivityItem };
