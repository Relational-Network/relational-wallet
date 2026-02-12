// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Relational Network

/**
 * Typed API client for the Relational Wallet enclave backend.
 *
 * This module provides a typed interface to the backend API using
 * types generated from the OpenAPI specification.
 */

import type { components, paths } from "@/types/api";

// =============================================================================
// Type Exports (derived from OpenAPI spec)
// =============================================================================

export type UserMeResponse = components["schemas"]["UserMeResponse"];
export type Role = components["schemas"]["Role"];
export type Bookmark = components["schemas"]["Bookmark"];
export type CreateBookmarkRequest = components["schemas"]["CreateBookmarkRequest"];
export type Invite = components["schemas"]["Invite"];
export type RedeemInviteRequest = components["schemas"]["RedeemInviteRequest"];
export type RecurringPayment = components["schemas"]["RecurringPayment"];
export type CreateRecurringPaymentRequest = components["schemas"]["CreateRecurringPaymentRequest"];
export type UpdateRecurringPaymentRequest = components["schemas"]["UpdateRecurringPaymentRequest"];
export type UpdateLastPaidDateRequest = components["schemas"]["UpdateLastPaidDateRequest"];
export type HealthResponse = components["schemas"]["HealthResponse"];
export type ReadyResponse = components["schemas"]["ReadyResponse"];

// Wallet types from backend
export type WalletResponse = components["schemas"]["WalletResponse"];
export type WalletListResponse = components["schemas"]["WalletListResponse"];
export type CreateWalletRequest = components["schemas"]["CreateWalletRequest"];
export type CreateWalletResponse = components["schemas"]["CreateWalletResponse"];
export type DeleteWalletResponse = components["schemas"]["DeleteWalletResponse"];
export type WalletStatus = components["schemas"]["WalletStatus"];

// Transaction types (manually defined until OpenAPI is regenerated)
export interface EstimateGasRequest {
  to: string;
  amount: string;
  token: string; // "native" or token contract address
  network?: string; // "fuji" or "mainnet"
}

export interface EstimateGasResponse {
  gas_limit: string;
  max_fee_per_gas: string;
  max_priority_fee_per_gas: string;
  estimated_cost_wei: string;
  estimated_cost: string;
}

export interface SendTransactionRequest {
  to: string;
  amount: string;
  token: string; // "native" or token contract address
  network?: string;
  gas_limit?: string;
  max_priority_fee_per_gas?: string;
}

export interface SendTransactionResponse {
  tx_hash: string;
  status: "pending" | "confirmed" | "failed";
  explorer_url: string;
}

export interface TransactionSummary {
  tx_hash: string;
  status: "pending" | "confirmed" | "failed";
  direction: "sent" | "received";
  from: string;
  to: string;
  amount: string;
  token: string;
  network: string;
  block_number?: number;
  explorer_url: string;
  timestamp: string;
}

export interface TransactionListResponse {
  transactions: TransactionSummary[];
}

export interface TransactionStatusResponse {
  tx_hash: string;
  status: "pending" | "confirmed" | "failed";
  block_number?: number;
  confirmations?: number;
  gas_used?: string;
  timestamp?: string;
}

export type FiatDirection = "on_ramp" | "off_ramp";
export type FiatRequestStatus = "queued" | "provider_pending" | "completed" | "failed";

export interface CreateFiatRequest {
  wallet_id: string;
  amount_eur: string;
  provider?: string;
  note?: string;
  beneficiary_account_holder_name?: string;
  beneficiary_iban?: string;
}

export interface FiatRequest {
  request_id: string;
  wallet_id: string;
  direction: FiatDirection;
  amount_eur: string;
  provider: string;
  status: FiatRequestStatus;
  note?: string;
  provider_reference?: string;
  provider_action_url?: string;
  created_at: string;
  updated_at: string;
}

export interface FiatRequestListResponse {
  requests: FiatRequest[];
  total: number;
}

export interface FiatProviderSummary {
  provider_id: string;
  display_name: string;
  sandbox: boolean;
  enabled: boolean;
  supports_on_ramp: boolean;
  supports_off_ramp: boolean;
}

export interface FiatProviderListResponse {
  default_provider: string;
  providers: FiatProviderSummary[];
}

// =============================================================================
// API Response Types
// =============================================================================

export type ApiResponse<T> =
  | { success: true; data: T }
  | { success: false; error: ApiError };

export interface ApiError {
  status: number;
  message: string;
}

// =============================================================================
// Configuration
// =============================================================================

/**
 * Determine the API base URL based on the environment.
 *
 * Browser-side calls go through the Next.js proxy route to avoid
 * certificate issues with the self-signed RA-TLS certificate.
 *
 * Server-side calls can go directly to the backend if needed.
 */
function getApiBaseUrl(): string {
  // In browser, always use the proxy route
  if (typeof window !== "undefined") {
    return "/api/proxy";
  }
  // Server-side can use direct backend URL
  return process.env.WALLET_API_BASE_URL || "https://localhost:8080";
}

// =============================================================================
// API Client Class
// =============================================================================

/**
 * Typed API client for the wallet backend.
 *
 * All methods accept a token parameter for authentication.
 *
 * In the browser, requests are proxied through `/api/proxy/[...path]`
 * to handle the self-signed RA-TLS certificate.
 */
export class WalletApiClient {
  private baseUrl: string;

  constructor(baseUrl?: string) {
    this.baseUrl = baseUrl || getApiBaseUrl();
  }

  /**
   * Make an authenticated API request.
   * @internal
   */
  private async request<T>(
    endpoint: string,
    options: RequestInit & { token?: string }
  ): Promise<ApiResponse<T>> {
    const { token, ...fetchOptions } = options;

    try {
      const headers: HeadersInit = {
        "Content-Type": "application/json",
        ...(fetchOptions.headers as Record<string, string>),
      };

      if (token) {
        headers["Authorization"] = `Bearer ${token}`;
      }

      const response = await fetch(`${this.baseUrl}${endpoint}`, {
        ...fetchOptions,
        headers,
      });

      if (!response.ok) {
        const text = await response.text();
        return {
          success: false,
          error: {
            status: response.status,
            message: text || `HTTP ${response.status}`,
          },
        };
      }

      // Handle 204 No Content
      if (response.status === 204) {
        return { success: true, data: undefined as T };
      }

      const data = await response.json();
      return { success: true, data };
    } catch (error) {
      return {
        success: false,
        error: {
          status: 0,
          message: error instanceof Error ? error.message : "Network error",
        },
      };
    }
  }

  // ===========================================================================
  // Health Endpoints (no auth required)
  // ===========================================================================

  /**
   * Check API health status.
   * This endpoint does not require authentication.
   */
  async checkHealth(): Promise<ApiResponse<ReadyResponse>> {
    return this.request<ReadyResponse>("/health", { method: "GET" });
  }

  /**
   * Check API liveness.
   */
  async checkLiveness(): Promise<ApiResponse<HealthResponse>> {
    return this.request<HealthResponse>("/health/live", { method: "GET" });
  }

  /**
   * Check API readiness.
   */
  async checkReadiness(): Promise<ApiResponse<ReadyResponse>> {
    return this.request<ReadyResponse>("/health/ready", { method: "GET" });
  }

  // ===========================================================================
  // User Endpoints
  // ===========================================================================

  /**
   * Get the current authenticated user's information.
   */
  async getCurrentUser(token: string): Promise<ApiResponse<UserMeResponse>> {
    return this.request<UserMeResponse>("/v1/users/me", {
      method: "GET",
      token,
    });
  }

  // ===========================================================================
  // Wallet Endpoints
  // ===========================================================================

  /**
   * List all wallets for the authenticated user.
   */
  async listWallets(token: string): Promise<ApiResponse<WalletListResponse>> {
    return this.request<WalletListResponse>("/v1/wallets", {
      method: "GET",
      token,
    });
  }

  /**
   * Get a specific wallet by ID.
   */
  async getWallet(token: string, walletId: string): Promise<ApiResponse<WalletResponse>> {
    return this.request<WalletResponse>(`/v1/wallets/${encodeURIComponent(walletId)}`, {
      method: "GET",
      token,
    });
  }

  /**
   * Create a new wallet for the authenticated user.
   */
  async createWallet(
    token: string,
    data: CreateWalletRequest
  ): Promise<ApiResponse<CreateWalletResponse>> {
    return this.request<CreateWalletResponse>("/v1/wallets", {
      method: "POST",
      token,
      body: JSON.stringify(data),
    });
  }

  /**
   * Delete (soft-delete) a wallet.
   */
  async deleteWallet(
    token: string,
    walletId: string
  ): Promise<ApiResponse<DeleteWalletResponse>> {
    return this.request<DeleteWalletResponse>(`/v1/wallets/${encodeURIComponent(walletId)}`, {
      method: "DELETE",
      token,
    });
  }

  // ===========================================================================
  // Transaction Endpoints
  // ===========================================================================

  /**
   * Estimate gas cost for a transaction.
   */
  async estimateGas(
    token: string,
    walletId: string,
    data: EstimateGasRequest
  ): Promise<ApiResponse<EstimateGasResponse>> {
    return this.request<EstimateGasResponse>(
      `/v1/wallets/${encodeURIComponent(walletId)}/estimate`,
      {
        method: "POST",
        token,
        body: JSON.stringify(data),
      }
    );
  }

  /**
   * Send a transaction from a wallet.
   */
  async sendTransaction(
    token: string,
    walletId: string,
    data: SendTransactionRequest
  ): Promise<ApiResponse<SendTransactionResponse>> {
    return this.request<SendTransactionResponse>(
      `/v1/wallets/${encodeURIComponent(walletId)}/send`,
      {
        method: "POST",
        token,
        body: JSON.stringify(data),
      }
    );
  }

  /**
   * List transaction history for a wallet.
   */
  async listTransactions(
    token: string,
    walletId: string,
    network?: string
  ): Promise<ApiResponse<TransactionListResponse>> {
    const params = network ? `?network=${encodeURIComponent(network)}` : "";
    return this.request<TransactionListResponse>(
      `/v1/wallets/${encodeURIComponent(walletId)}/transactions${params}`,
      {
        method: "GET",
        token,
      }
    );
  }

  /**
   * Get status of a specific transaction (for polling).
   */
  async getTransactionStatus(
    token: string,
    walletId: string,
    txHash: string
  ): Promise<ApiResponse<TransactionStatusResponse>> {
    return this.request<TransactionStatusResponse>(
      `/v1/wallets/${encodeURIComponent(walletId)}/transactions/${encodeURIComponent(txHash)}`,
      {
        method: "GET",
        token,
      }
    );
  }

  // ===========================================================================
  // Fiat Endpoints (stubs)
  // ===========================================================================

  /**
   * List supported fiat providers.
   */
  async listFiatProviders(
    token: string
  ): Promise<ApiResponse<FiatProviderListResponse>> {
    return this.request<FiatProviderListResponse>("/v1/fiat/providers", {
      method: "GET",
      token,
    });
  }

  /**
   * Create a fiat on-ramp request.
   */
  async createFiatOnRampRequest(
    token: string,
    data: CreateFiatRequest
  ): Promise<ApiResponse<FiatRequest>> {
    return this.request<FiatRequest>("/v1/fiat/onramp/requests", {
      method: "POST",
      token,
      body: JSON.stringify(data),
    });
  }

  /**
   * Create a fiat off-ramp request.
   */
  async createFiatOffRampRequest(
    token: string,
    data: CreateFiatRequest
  ): Promise<ApiResponse<FiatRequest>> {
    return this.request<FiatRequest>("/v1/fiat/offramp/requests", {
      method: "POST",
      token,
      body: JSON.stringify(data),
    });
  }

  /**
   * List fiat requests for current user, optionally by wallet.
   */
  async listFiatRequests(
    token: string,
    walletId?: string
  ): Promise<ApiResponse<FiatRequestListResponse>> {
    const params = walletId ? `?wallet_id=${encodeURIComponent(walletId)}` : "";
    return this.request<FiatRequestListResponse>(`/v1/fiat/requests${params}`, {
      method: "GET",
      token,
    });
  }

  /**
   * Get a fiat request by ID.
   */
  async getFiatRequest(
    token: string,
    requestId: string
  ): Promise<ApiResponse<FiatRequest>> {
    return this.request<FiatRequest>(`/v1/fiat/requests/${encodeURIComponent(requestId)}`, {
      method: "GET",
      token,
    });
  }

  // ===========================================================================
  // Bookmark Endpoints
  // ===========================================================================

  /**
   * List bookmarks for a wallet.
   */
  async listBookmarks(
    token: string,
    walletId: string
  ): Promise<ApiResponse<Bookmark[]>> {
    return this.request<Bookmark[]>(`/v1/bookmarks?wallet_id=${encodeURIComponent(walletId)}`, {
      method: "GET",
      token,
    });
  }

  /**
   * Create a new bookmark.
   */
  async createBookmark(
    token: string,
    data: CreateBookmarkRequest
  ): Promise<ApiResponse<Bookmark>> {
    return this.request<Bookmark>("/v1/bookmarks", {
      method: "POST",
      token,
      body: JSON.stringify(data),
    });
  }

  /**
   * Delete a bookmark.
   */
  async deleteBookmark(
    token: string,
    bookmarkId: string
  ): Promise<ApiResponse<void>> {
    return this.request<void>(`/v1/bookmarks/${encodeURIComponent(bookmarkId)}`, {
      method: "DELETE",
      token,
    });
  }

  // ===========================================================================
  // Invite Endpoints
  // ===========================================================================

  /**
   * Get invite details by code.
   */
  async getInvite(token: string, inviteCode: string): Promise<ApiResponse<Invite>> {
    return this.request<Invite>(`/v1/invite?invite_code=${encodeURIComponent(inviteCode)}`, {
      method: "GET",
      token,
    });
  }

  /**
   * Redeem an invite code.
   */
  async redeemInvite(
    token: string,
    data: RedeemInviteRequest
  ): Promise<ApiResponse<void>> {
    return this.request<void>("/v1/invite/redeem", {
      method: "POST",
      token,
      body: JSON.stringify(data),
    });
  }

  // ===========================================================================
  // Recurring Payment Endpoints
  // ===========================================================================

  /**
   * List recurring payments for a wallet.
   */
  async listRecurringPayments(
    token: string,
    walletId: string
  ): Promise<ApiResponse<RecurringPayment[]>> {
    return this.request<RecurringPayment[]>(`/v1/recurring/payments?wallet_id=${encodeURIComponent(walletId)}`, {
      method: "GET",
      token,
    });
  }

  /**
   * Create a recurring payment.
   */
  async createRecurringPayment(
    token: string,
    data: CreateRecurringPaymentRequest
  ): Promise<ApiResponse<void>> {
    return this.request<void>("/v1/recurring/payment", {
      method: "POST",
      token,
      body: JSON.stringify(data),
    });
  }

  /**
   * Update a recurring payment.
   */
  async updateRecurringPayment(
    token: string,
    paymentId: string,
    data: Omit<UpdateRecurringPaymentRequest, "recurring_payment_id">
  ): Promise<ApiResponse<void>> {
    return this.request<void>(`/v1/recurring/payment/${encodeURIComponent(paymentId)}`, {
      method: "PUT",
      token,
      body: JSON.stringify({ ...data, recurring_payment_id: paymentId }),
    });
  }

  /**
   * Delete a recurring payment.
   */
  async deleteRecurringPayment(
    token: string,
    paymentId: string
  ): Promise<ApiResponse<void>> {
    return this.request<void>(`/v1/recurring/payment/${encodeURIComponent(paymentId)}`, {
      method: "DELETE",
      token,
    });
  }

  /**
   * Get recurring payments due today.
   */
  async getRecurringPaymentsToday(
    token: string
  ): Promise<ApiResponse<RecurringPayment[]>> {
    return this.request<RecurringPayment[]>("/v1/recurring/payments/today", {
      method: "GET",
      token,
    });
  }
}

// =============================================================================
// Singleton Instance
// =============================================================================

export const apiClient = new WalletApiClient();

// =============================================================================
// Path Type Exports (for advanced usage)
// =============================================================================

export type { paths };
