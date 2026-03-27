// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import {
  Activity,
  BarChart3,
  Wallet,
  Shield,
  RefreshCw,
  Copy,
  Check,
  ExternalLink,
  ArrowLeft,
  AlertCircle,
  ChevronLeft,
  ChevronRight,
  Search,
  X,
  CheckCircle2,
  XCircle,
  Clock,
} from "lucide-react";

// ─── Types ───────────────────────────────────────────────────────────────────

interface SystemStats {
  total_wallets: number;
  active_wallets: number;
  suspended_wallets: number;
  deleted_wallets: number;
  total_bookmarks: number;
  uptime_seconds: number;
  timestamp: string;
}

interface AdminWalletItem {
  wallet_id: string;
  owner_user_id: string;
  public_address: string;
  status: string;
  created_at: string;
  label?: string;
}

interface AuditEvent {
  event_id: string;
  timestamp: string;
  event_type: string;
  user_id?: string;
  resource_id?: string;
  resource_type?: string;
  success: boolean;
  error?: string;
  details?: string;
  ip_address?: string;
}

interface AuditLogResponse {
  events: AuditEvent[];
  has_more: boolean;
  total: number;
}

interface ServiceWalletStatus {
  wallet_id: string;
  public_address: string;
  bootstrapped: boolean;
  chain_network: string;
  reur_contract_address: string;
  avax_balance: string;
  reur_balance: string;
  reur_balance_raw: string;
}

interface DetailedHealth {
  status: string;
  storage: {
    data_dir: string;
    exists: boolean;
    writable: boolean;
    total_files: number;
  };
  auth_configured: boolean;
  version: string;
  build_time: string;
}

interface ServiceCheck {
  name: string;
  status: "operational" | "degraded" | "down" | "checking";
  latency?: number;
  detail?: string;
}

type Tab = "status" | "wallets" | "audit" | "fiat";

// ─── Helpers ─────────────────────────────────────────────────────────────────

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatBalance(value: string): string {
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) return value;
  return parsed.toLocaleString(undefined, {
    minimumFractionDigits: 0,
    maximumFractionDigits: 6,
  });
}

function truncateAddr(addr: string): string {
  if (addr.length <= 14) return addr;
  return `${addr.slice(0, 8)}…${addr.slice(-6)}`;
}

/** Human-readable event type labels */
const EVENT_LABELS: Record<string, string> = {
  wallet_created: "Wallet Created",
  wallet_accessed: "Wallet Accessed",
  wallet_deleted: "Wallet Deleted",
  wallet_suspended: "Wallet Suspended",
  wallet_activated: "Wallet Activated",
  fiat_on_ramp_requested: "Fiat On-Ramp Request",
  fiat_off_ramp_requested: "Fiat Off-Ramp Request",
  transaction_sent: "Transaction Sent",
  transaction_signed: "Transaction Signed",
  bookmark_created: "Bookmark Created",
  bookmark_deleted: "Bookmark Deleted",
  balance_queried: "Balance Queried",
  gas_estimated: "Gas Estimated",
  admin_action: "Admin Action",
};

function humanEventType(raw: string): string {
  return EVENT_LABELS[raw] ?? raw.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

/** Relative time like "2 min ago", "3h ago" */
function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const sec = Math.floor(diff / 1000);
  if (sec < 60) return "just now";
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const days = Math.floor(hr / 24);
  return `${days}d ago`;
}

async function adminFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api/proxy${path}`, init);
  if (!res.ok) {
    const text = await res.text().catch(() => "Unknown error");
    throw new Error(`${res.status}: ${text}`);
  }
  return res.json();
}

async function adminPost(path: string): Promise<void> {
  const res = await fetch(`/api/proxy${path}`, { method: "POST" });
  if (!res.ok) {
    const text = await res.text().catch(() => "Unknown error");
    throw new Error(`${res.status}: ${text}`);
  }
}

// ─── Stat card ───────────────────────────────────────────────────────────────

function Stat({
  label,
  value,
  sub,
}: {
  label: string;
  value: string | number;
  sub?: string;
}) {
  return (
    <div className="card card-pad" style={{ flex: "1 1 140px", minWidth: 140 }}>
      <div style={{ fontSize: "0.8125rem", color: "var(--ink-secondary)", marginBottom: 4 }}>
        {label}
      </div>
      <div style={{ fontSize: "1.5rem", fontWeight: 700, color: "var(--ink)" }}>{value}</div>
      {sub && (
        <div style={{ fontSize: "0.75rem", color: "var(--ink-muted)", marginTop: 4 }}>
          {sub}
        </div>
      )}
    </div>
  );
}

// ─── Tab button ──────────────────────────────────────────────────────────────

function TabBtn({
  active,
  icon,
  label,
  onClick,
}: {
  active: boolean;
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={active ? "btn btn-primary" : "btn btn-ghost"}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 8,
        fontSize: "0.875rem",
        padding: "0.5rem 1rem",
      }}
    >
      {icon}
      {label}
    </button>
  );
}

// ─── Loading spinner text ────────────────────────────────────────────────────

function Loading() {
  return (
    <p style={{ color: "var(--ink-muted)", fontSize: "0.9375rem", padding: "2rem 0", textAlign: "center" }}>
      Loading…
    </p>
  );
}

// ─── Shared error banner ─────────────────────────────────────────────────────

function ErrorBanner({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="alert alert-error" style={{ display: "flex", alignItems: "center", gap: 10 }}>
      <AlertCircle size={18} />
      <span style={{ flex: 1 }}>{message}</span>
      {onRetry && (
        <button
          type="button"
          onClick={onRetry}
          className="btn btn-ghost"
          style={{ fontSize: "0.8125rem" }}
        >
          Retry
        </button>
      )}
    </div>
  );
}

// ─── Service status indicator ────────────────────────────────────────────────

function StatusDot({ status }: { status: ServiceCheck["status"] }) {
  const colors: Record<string, string> = {
    operational: "var(--success)",
    degraded: "var(--warning)",
    down: "var(--danger)",
    checking: "var(--ink-muted)",
  };
  return (
    <span
      style={{
        display: "inline-block",
        width: 10,
        height: 10,
        borderRadius: "50%",
        background: colors[status] ?? "var(--ink-muted)",
        flexShrink: 0,
      }}
    />
  );
}

// ─── Status / Availability tab ───────────────────────────────────────────────

function StatusTab() {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [health, setHealth] = useState<DetailedHealth | null>(null);
  const [checks, setChecks] = useState<ServiceCheck[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lastChecked, setLastChecked] = useState<Date | null>(null);

  const runChecks = useCallback(async () => {
    setLoading(true);
    setError(null);

    const results: ServiceCheck[] = [
      { name: "Enclave API", status: "checking" },
      { name: "Encrypted Storage", status: "checking" },
      { name: "Authentication (JWKS)", status: "checking" },
      { name: "Fiat Service Wallet", status: "checking" },
    ];
    setChecks([...results]);

    // Check 1: Enclave API (stats)
    try {
      const t0 = performance.now();
      const s = await adminFetch<SystemStats>("/v1/admin/stats");
      const latency = Math.round(performance.now() - t0);
      setStats(s);
      results[0] = { name: "Enclave API", status: "operational", latency, detail: `${s.total_wallets} wallets` };
    } catch {
      results[0] = { name: "Enclave API", status: "down", detail: "Unreachable" };
    }
    setChecks([...results]);

    // Check 2: Storage (health)
    try {
      const t0 = performance.now();
      const h = await adminFetch<DetailedHealth>("/v1/admin/health");
      const latency = Math.round(performance.now() - t0);
      setHealth(h);
      const ok = h.storage.exists && h.storage.writable;
      results[1] = {
        name: "Encrypted Storage",
        status: ok ? "operational" : "degraded",
        latency,
        detail: ok ? "Healthy" : "Read-only or missing",
      };
      results[2] = {
        name: "Authentication (JWKS)",
        status: h.auth_configured ? "operational" : "degraded",
        detail: h.auth_configured ? "Verified" : "Development mode",
      };
    } catch {
      results[1] = { name: "Encrypted Storage", status: "down", detail: "Health check failed" };
      results[2] = { name: "Authentication (JWKS)", status: "down", detail: "Health check failed" };
    }
    setChecks([...results]);

    // Check 3: Fiat service wallet
    try {
      const t0 = performance.now();
      await adminFetch<ServiceWalletStatus>("/v1/admin/fiat/service-wallet");
      const latency = Math.round(performance.now() - t0);
      results[3] = { name: "Fiat Service Wallet", status: "operational", latency, detail: "Bootstrapped" };
    } catch {
      results[3] = { name: "Fiat Service Wallet", status: "degraded", detail: "Unavailable" };
    }
    setChecks([...results]);

    setLastChecked(new Date());
    setLoading(false);
  }, []);

  useEffect(() => {
    runChecks();
  }, [runChecks]);

  const allOperational = checks.every((c) => c.status === "operational");
  const anyDown = checks.some((c) => c.status === "down");
  const overallStatus = loading
    ? "Checking…"
    : anyDown
      ? "Partial Outage"
      : allOperational
        ? "All Systems Operational"
        : "Degraded Performance";
  const overallColor = loading
    ? "var(--ink-muted)"
    : anyDown
      ? "var(--danger)"
      : allOperational
        ? "var(--success)"
        : "var(--warning)";

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Overall banner */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "1rem 1.25rem",
          borderRadius: "var(--radius-md)",
          background: loading ? "var(--bg-subtle)" : anyDown ? "var(--danger-light)" : allOperational ? "var(--success-light)" : "var(--warning-light)",
          border: `1px solid ${loading ? "var(--border)" : anyDown ? "rgba(220,38,38,0.2)" : allOperational ? "rgba(22,163,74,0.2)" : "rgba(217,119,6,0.2)"}`,
        }}
      >
        {loading ? (
          <Clock size={22} color={overallColor} />
        ) : allOperational ? (
          <CheckCircle2 size={22} color={overallColor} />
        ) : (
          <XCircle size={22} color={overallColor} />
        )}
        <span style={{ fontSize: "1.125rem", fontWeight: 700, color: overallColor }}>{overallStatus}</span>
        <span style={{ marginLeft: "auto", fontSize: "0.75rem", color: "var(--ink-muted)" }}>
          {lastChecked ? `Checked ${lastChecked.toLocaleTimeString()}` : ""}
        </span>
        <button type="button" className="btn btn-ghost" onClick={runChecks} style={{ padding: "0.25rem 0.5rem" }}>
          <RefreshCw size={14} />
        </button>
      </div>

      {/* Service checks */}
      <div className="card" style={{ overflow: "hidden" }}>
        {checks.map((c, i) => (
          <div
            key={c.name}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 12,
              padding: "0.875rem 1.25rem",
              borderBottom: i < checks.length - 1 ? "1px solid var(--border)" : undefined,
            }}
          >
            <StatusDot status={c.status} />
            <span style={{ fontWeight: 600, fontSize: "0.9375rem", color: "var(--ink)", flex: 1 }}>{c.name}</span>
            {c.detail && (
              <span style={{ fontSize: "0.8125rem", color: "var(--ink-secondary)" }}>{c.detail}</span>
            )}
            {c.latency !== undefined && (
              <span
                style={{
                  fontSize: "0.75rem",
                  fontFamily: "var(--font-mono)",
                  color: c.latency < 500 ? "var(--ink-muted)" : "var(--warning)",
                  minWidth: 52,
                  textAlign: "right",
                }}
              >
                {c.latency}ms
              </span>
            )}
          </div>
        ))}
      </div>

      {/* Quick stats row */}
      {stats && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: "1rem" }}>
          <Stat label="Total Wallets" value={stats.total_wallets} />
          <Stat label="Active" value={stats.active_wallets} />
          <Stat label="Suspended" value={stats.suspended_wallets} />
          <Stat label="Uptime" value={formatUptime(stats.uptime_seconds)} />
        </div>
      )}

      {/* Build info */}
      {health && (
        <div style={{ fontSize: "0.75rem", color: "var(--ink-muted)", display: "flex", gap: "2rem", flexWrap: "wrap" }}>
          <span>Version: {health.version}</span>
          {health.build_time && <span>Built: {health.build_time}</span>}
        </div>
      )}

      {error && <ErrorBanner message={error} onRetry={runChecks} />}
    </div>
  );
}

// ─── Wallets tab ─────────────────────────────────────────────────────────────

function WalletsTab() {
  const [wallets, setWallets] = useState<AdminWalletItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await adminFetch<{ wallets: AdminWalletItem[] }>("/v1/admin/wallets");
      setWallets(data.wallets);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load");
    } finally {
      setLoading(false);
    }
  }, []);

  const toggleStatus = useCallback(async (wallet: AdminWalletItem) => {
    const action = wallet.status === "active" ? "suspend" : "activate";
    const confirmMsg = action === "suspend"
      ? `Suspend wallet ${truncateAddr(wallet.public_address)}? The owner will not be able to transact.`
      : `Activate wallet ${truncateAddr(wallet.public_address)}?`;

    if (!window.confirm(confirmMsg)) return;

    setActionLoading(wallet.wallet_id);
    try {
      await adminPost(`/v1/admin/wallets/${wallet.wallet_id}/${action}`);
      await load();
    } catch (e) {
      alert(`Failed to ${action}: ${e instanceof Error ? e.message : "error"}`);
    } finally {
      setActionLoading(null);
    }
  }, [load]);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) return <Loading />;
  if (error) return <ErrorBanner message={error} onRetry={load} />;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontSize: "0.8125rem", color: "var(--ink-muted)" }}>
          {wallets.length} wallet{wallets.length !== 1 ? "s" : ""}
        </span>
        <button type="button" className="btn btn-ghost" onClick={load} style={{ padding: "0.25rem 0.5rem" }}>
          <RefreshCw size={14} />
        </button>
      </div>

      <div style={{ overflowX: "auto" }}>
        <table style={{ width: "100%", fontSize: "0.9375rem", borderCollapse: "collapse", color: "var(--ink)" }}>
          <thead>
            <tr style={{ borderBottom: "2px solid var(--border-strong)" }}>
              <th style={{ padding: "0.75rem 1rem", textAlign: "left", color: "var(--ink-secondary)", fontWeight: 600 }}>Address</th>
              <th style={{ padding: "0.75rem 1rem", textAlign: "left", color: "var(--ink-secondary)", fontWeight: 600 }}>Status</th>
              <th style={{ padding: "0.75rem 1rem", textAlign: "left", color: "var(--ink-secondary)", fontWeight: 600 }}>Created</th>
              <th style={{ padding: "0.75rem 1rem", textAlign: "right", color: "var(--ink-secondary)", fontWeight: 600 }}>Action</th>
            </tr>
          </thead>
          <tbody>
            {wallets.map((w) => (
              <tr key={w.wallet_id} style={{ borderBottom: "1px solid var(--border)" }}>
                <td style={{ padding: "0.75rem 1rem" }}>
                  <a
                    href={`https://testnet.snowtrace.io/address/${w.public_address}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{ color: "var(--brand)", fontFamily: "var(--font-mono)", fontSize: "0.8125rem", display: "inline-flex", alignItems: "center", gap: 4 }}
                  >
                    {truncateAddr(w.public_address)}
                    <ExternalLink size={12} />
                  </a>
                </td>
                <td style={{ padding: "0.75rem 1rem" }}>
                  <span className={`badge ${w.status === "active" ? "badge-success" : w.status === "suspended" ? "badge-warning" : "badge-danger"}`}>
                    {w.status}
                  </span>
                </td>
                <td style={{ padding: "0.75rem 1rem", color: "var(--ink-secondary)", fontSize: "0.875rem" }}>
                  {new Date(w.created_at).toLocaleDateString()}
                </td>
                <td style={{ padding: "0.75rem 1rem", textAlign: "right" }}>
                  {(w.status === "active" || w.status === "suspended") && (
                    <button
                      type="button"
                      className={w.status === "active" ? "btn btn-danger" : "btn btn-secondary"}
                      style={{ fontSize: "0.75rem", padding: "0.3rem 0.75rem" }}
                      disabled={actionLoading === w.wallet_id}
                      onClick={() => toggleStatus(w)}
                    >
                      {actionLoading === w.wallet_id
                        ? "…"
                        : w.status === "active"
                          ? "Suspend"
                          : "Activate"}
                    </button>
                  )}
                </td>
              </tr>
            ))}
            {wallets.length === 0 && (
              <tr>
                <td colSpan={4} style={{ padding: "2rem 1rem", color: "var(--ink-muted)", textAlign: "center" }}>
                  No wallets found
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

// ─── Audit tab ───────────────────────────────────────────────────────────────

const PAGE_SIZE = 20;

function AuditTab() {
  const [events, setEvents] = useState<AuditEvent[]>([]);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(false);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Filters
  const [filterType, setFilterType] = useState("");
  const [filterStatus, setFilterStatus] = useState<"" | "success" | "fail">("");
  const [filterSearch, setFilterSearch] = useState("");
  const searchRef = useRef<HTMLInputElement>(null);

  const load = useCallback(async (pageNum: number) => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams();
      params.set("limit", String(PAGE_SIZE));
      params.set("offset", String(pageNum * PAGE_SIZE));
      if (filterType) params.set("event_type", filterType);
      const data = await adminFetch<AuditLogResponse>(`/v1/admin/audit/events?${params}`);
      setEvents(data.events);
      setTotal(data.total);
      setHasMore(data.has_more);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load");
    } finally {
      setLoading(false);
    }
  }, [filterType]);

  useEffect(() => {
    setPage(0);
    load(0);
  }, [load]);

  const goPage = useCallback((p: number) => {
    setPage(p);
    load(p);
  }, [load]);

  // Client-side filtering for status & search (server does event_type + pagination)
  const filtered = useMemo(() => {
    let list = events;
    if (filterStatus === "success") list = list.filter((e) => e.success);
    if (filterStatus === "fail") list = list.filter((e) => !e.success);
    if (filterSearch.trim()) {
      const q = filterSearch.toLowerCase();
      list = list.filter(
        (e) =>
          humanEventType(e.event_type).toLowerCase().includes(q) ||
          (e.resource_id ?? "").toLowerCase().includes(q) ||
          (e.user_id ?? "").toLowerCase().includes(q)
      );
    }
    return list;
  }, [events, filterStatus, filterSearch]);

  // Collect unique event types for filter dropdown
  const eventTypes = useMemo(() => {
    const set = new Set(events.map((e) => e.event_type));
    return Array.from(set).sort();
  }, [events]);

  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1rem" }}>
      {/* Filters bar */}
      <div style={{ display: "flex", gap: "0.5rem", flexWrap: "wrap", alignItems: "center" }}>
        {/* Search */}
        <div style={{ position: "relative", flex: "1 1 200px", minWidth: 180 }}>
          <Search size={14} style={{ position: "absolute", left: 10, top: "50%", transform: "translateY(-50%)", color: "var(--ink-muted)" }} />
          <input
            ref={searchRef}
            type="text"
            placeholder="Search events…"
            value={filterSearch}
            onChange={(e) => setFilterSearch(e.target.value)}
            className="input"
            style={{ paddingLeft: 32, fontSize: "0.8125rem", height: 36 }}
          />
          {filterSearch && (
            <button
              type="button"
              onClick={() => setFilterSearch("")}
              style={{ position: "absolute", right: 8, top: "50%", transform: "translateY(-50%)", background: "none", border: "none", cursor: "pointer", color: "var(--ink-muted)", padding: 2 }}
            >
              <X size={14} />
            </button>
          )}
        </div>

        {/* Event type filter */}
        <select
          value={filterType}
          onChange={(e) => setFilterType(e.target.value)}
          className="input"
          style={{ fontSize: "0.8125rem", height: 36, width: "auto", minWidth: 160 }}
        >
          <option value="">All event types</option>
          {eventTypes.map((t) => (
            <option key={t} value={t}>{humanEventType(t)}</option>
          ))}
        </select>

        {/* Status filter */}
        <select
          value={filterStatus}
          onChange={(e) => setFilterStatus(e.target.value as "" | "success" | "fail")}
          className="input"
          style={{ fontSize: "0.8125rem", height: 36, width: "auto", minWidth: 120 }}
        >
          <option value="">All statuses</option>
          <option value="success">Success</option>
          <option value="fail">Failed</option>
        </select>

        <button type="button" className="btn btn-ghost" onClick={() => goPage(page)} style={{ padding: "0.25rem 0.5rem" }}>
          <RefreshCw size={14} />
        </button>
      </div>

      {loading ? (
        <Loading />
      ) : error ? (
        <ErrorBanner message={error} onRetry={() => goPage(page)} />
      ) : (
        <>
          <div style={{ overflowX: "auto" }}>
            <table style={{ width: "100%", fontSize: "0.875rem", borderCollapse: "collapse", color: "var(--ink)" }}>
              <thead>
                <tr style={{ borderBottom: "2px solid var(--border-strong)" }}>
                  <th style={{ padding: "0.75rem 1rem", textAlign: "left", color: "var(--ink-secondary)", fontWeight: 600 }}>Time</th>
                  <th style={{ padding: "0.75rem 1rem", textAlign: "left", color: "var(--ink-secondary)", fontWeight: 600 }}>Event</th>
                  <th style={{ padding: "0.75rem 1rem", textAlign: "left", color: "var(--ink-secondary)", fontWeight: 600 }}>Resource</th>
                  <th style={{ padding: "0.75rem 1rem", textAlign: "left", color: "var(--ink-secondary)", fontWeight: 600 }}>Status</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((ev) => (
                  <tr key={ev.event_id} style={{ borderBottom: "1px solid var(--border)" }}>
                    <td style={{ padding: "0.75rem 1rem", whiteSpace: "nowrap" }}>
                      <span title={new Date(ev.timestamp).toLocaleString()} style={{ cursor: "default" }}>
                        {timeAgo(ev.timestamp)}
                      </span>
                    </td>
                    <td style={{ padding: "0.75rem 1rem", fontWeight: 500 }}>
                      {humanEventType(ev.event_type)}
                    </td>
                    <td style={{ padding: "0.75rem 1rem", fontFamily: "var(--font-mono)", fontSize: "0.8125rem", color: "var(--ink-secondary)" }}>
                      {ev.resource_id ? truncateAddr(ev.resource_id) : "—"}
                    </td>
                    <td style={{ padding: "0.75rem 1rem" }}>
                      <span className={`badge ${ev.success ? "badge-success" : "badge-danger"}`}>
                        {ev.success ? "Success" : "Failed"}
                      </span>
                    </td>
                  </tr>
                ))}
                {filtered.length === 0 && (
                  <tr>
                    <td colSpan={4} style={{ padding: "2rem 1rem", color: "var(--ink-muted)", textAlign: "center" }}>
                      {events.length === 0 ? "No audit events" : "No matching events"}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", fontSize: "0.8125rem", color: "var(--ink-secondary)" }}>
            <span>{total} total event{total !== 1 ? "s" : ""}</span>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <button
                type="button"
                className="btn btn-ghost"
                disabled={page === 0}
                onClick={() => goPage(page - 1)}
                style={{ padding: "0.25rem 0.5rem" }}
              >
                <ChevronLeft size={16} />
              </button>
              <span>
                Page {page + 1} of {totalPages}
              </span>
              <button
                type="button"
                className="btn btn-ghost"
                disabled={!hasMore}
                onClick={() => goPage(page + 1)}
                style={{ padding: "0.25rem 0.5rem" }}
              >
                <ChevronRight size={16} />
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

// ─── Fiat / Service Wallet tab ───────────────────────────────────────────────

function FiatTab() {
  const [data, setData] = useState<ServiceWalletStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const sw = await adminFetch<ServiceWalletStatus>("/v1/admin/fiat/service-wallet");
      setData(sw);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load");
    } finally {
      setLoading(false);
    }
  }, []);

  const handleCopy = useCallback(() => {
    if (!data) return;
    navigator.clipboard.writeText(data.public_address);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }, [data]);

  useEffect(() => {
    load();
  }, [load]);

  if (loading) return <Loading />;
  if (error) return <ErrorBanner message={error} onRetry={load} />;
  if (!data) return null;

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "1.25rem" }}>
      {/* Service Wallet Address */}
      <div className="card card-pad">
        <div style={{ fontSize: "0.8125rem", color: "var(--ink-secondary)", marginBottom: 8, fontWeight: 600 }}>
          Service Wallet Address
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <Wallet size={20} color="var(--ink-secondary)" />
          <code style={{ fontSize: "1rem", color: "var(--ink)", wordBreak: "break-all" }}>
            {data.public_address}
          </code>
          <button
            type="button"
            onClick={handleCopy}
            className="btn btn-icon"
            title="Copy address"
          >
            {copied ? <Check size={16} color="var(--success)" /> : <Copy size={16} />}
          </button>
          <a
            href={`https://testnet.snowtrace.io/address/${data.public_address}`}
            target="_blank"
            rel="noopener noreferrer"
            className="btn btn-icon"
            title="View on explorer"
          >
            <ExternalLink size={16} />
          </a>
        </div>
      </div>

      {/* Balances */}
      <div style={{ display: "flex", gap: "1rem", flexWrap: "wrap" }}>
        <Stat label="AVAX Balance" value={formatBalance(data.avax_balance)} />
        <Stat label="rEUR Balance" value={formatBalance(data.reur_balance)} />
        <Stat label="Network" value={data.chain_network} />
      </div>

      {/* Actions */}
      <div style={{ display: "flex", gap: "0.75rem", alignItems: "center", flexWrap: "wrap" }}>
        <button
          type="button"
          className="btn btn-secondary"
          onClick={() => load()}
        >
          <RefreshCw size={16} style={{ marginRight: 6 }} />
          Refresh Balances
        </button>
      </div>

      {/* Faucet Link */}
      <div className="card card-pad" style={{ fontSize: "0.875rem", color: "var(--ink-secondary)" }}>
        <strong style={{ color: "var(--ink)" }}>Fund Service Wallet</strong>
        <div style={{ marginTop: 8, display: "flex", gap: "1.5rem" }}>
          <a
            href="https://core.app/tools/testnet-faucet/?subnet=c&token=c"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: "var(--brand)", display: "flex", alignItems: "center", gap: 4 }}
          >
            AVAX Faucet <ExternalLink size={14} />
          </a>
        </div>
      </div>
    </div>
  );
}

// ─── Main Admin Panel ────────────────────────────────────────────────────────

export function AdminPanel() {
  const [tab, setTab] = useState<Tab>("status");

  return (
    <main className="app-container" style={{ maxWidth: 1200, margin: "0 auto", padding: "2rem 1.5rem" }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: "1rem", marginBottom: "1.5rem" }}>
        <Link
          href="/wallets"
          className="btn btn-icon"
          title="Back to wallets"
        >
          <ArrowLeft size={20} />
        </Link>
        <Shield size={24} color="var(--ink)" />
        <h1 style={{ fontSize: "1.5rem", fontWeight: 700, margin: 0, color: "var(--ink)" }}>Admin Panel</h1>
      </div>

      {/* Tab bar */}
      <div style={{
        display: "flex",
        gap: 6,
        marginBottom: "1.5rem",
        flexWrap: "wrap",
        background: "var(--bg-subtle)",
        borderRadius: 12,
        padding: 6,
        border: "1px solid var(--border)",
      }}>
        <TabBtn
          active={tab === "status"}
          icon={<Activity size={16} />}
          label="Status"
          onClick={() => setTab("status")}
        />
        <TabBtn
          active={tab === "wallets"}
          icon={<Wallet size={16} />}
          label="Wallets"
          onClick={() => setTab("wallets")}
        />
        <TabBtn
          active={tab === "audit"}
          icon={<Shield size={16} />}
          label="Audit Log"
          onClick={() => setTab("audit")}
        />
        <TabBtn
          active={tab === "fiat"}
          icon={<Wallet size={16} />}
          label="Fiat Service"
          onClick={() => setTab("fiat")}
        />
      </div>

      {/* Tab content */}
      <div className="card card-pad" style={{ minHeight: 300 }}>
        {tab === "status" && <StatusTab />}
        {tab === "wallets" && <WalletsTab />}
        {tab === "audit" && <AuditTab />}
        {tab === "fiat" && <FiatTab />}
      </div>
    </main>
  );
}
