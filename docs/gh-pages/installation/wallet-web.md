---
layout: default
title: Wallet Web
parent: Installation
nav_order: 2
---

# Wallet Web (Next.js Frontend)
{: .fs-7 }

The browser interface for Relational Wallet. Built with Next.js 16 and React 19, it provides wallet management, transaction flows, fiat on/off-ramp, and an admin dashboard --- all secured by Clerk authentication.
{: .fs-5 .fw-300 }

---

## Prerequisites

| Requirement | Version | Notes |
|:------------|:--------|:------|
| Node.js | 20+ | Runtime |
| pnpm | Latest | Package manager |
| Clerk account | --- | Authentication provider |
| Running backend | --- | Rust server on `https://localhost:8080` |

---

## Setup

### 1. Install Dependencies

```bash
cd apps/wallet-web
pnpm install
```

### 2. Configure Environment

Create `.env.local` in the `apps/wallet-web/` directory:

```env
# Clerk authentication (required)
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=pk_test_...
CLERK_SECRET_KEY=sk_test_...

# Backend URL (server-side only, never exposed to browser)
WALLET_API_BASE_URL=https://localhost:8080

# Accept self-signed RA-TLS certificates (development only)
NODE_TLS_REJECT_UNAUTHORIZED=0
```

`NODE_TLS_REJECT_UNAUTHORIZED=0` is **rejected** when `NODE_ENV=production`. Use proper CA-trusted certificates in production via the [reverse proxy](/relational-wallet/installation/rust-server#docker-sgx) or a load balancer.
{: .warning }

### 3. Optional Clerk Route Overrides

```env
NEXT_PUBLIC_CLERK_SIGN_IN_URL=/sign-in
NEXT_PUBLIC_CLERK_SIGN_UP_URL=/sign-up
```

### 4. Start Development Server

```bash
pnpm dev
# http://localhost:3000
```

---

## Route Map

| Route | Purpose | Auth Required |
|:------|:--------|:-------------|
| `/` | Landing page (redirects to `/wallets` if signed in) | No |
| `/sign-in` | Clerk sign-in | No |
| `/sign-up` | Clerk sign-up | No |
| `/wallets` | Wallet dashboard --- list, create, switch | Yes |
| `/wallets/new` | Create a new wallet | Yes |
| `/wallets/[wallet_id]` | Wallet detail --- balance, actions | Yes |
| `/wallets/[wallet_id]/send` | Send transaction | Yes |
| `/wallets/[wallet_id]/transactions` | Transaction history | Yes |
| `/wallets/[wallet_id]/fiat` | Fiat on-ramp / off-ramp | Yes |
| `/pay` | Payment request viewer (for payment links) | No |
| `/callback` | TrueLayer on-ramp callback handler | Yes |
| `/account` | User account page with JWT token display | Yes |
| `/admin` | Admin dashboard (stats, users, wallets, audit) | Yes (Admin role) |

---

## Proxy Architecture

The frontend uses a **server-side proxy** to communicate with the enclave backend. Browser requests never connect directly to the RA-TLS endpoint.

```
┌──────────┐    ┌────────────────────────┐    ┌──────────────────┐
│  Browser  │───►│  Next.js Server        │───►│  SGX Backend     │
│           │    │  /api/proxy/*          │    │  :8080 (RA-TLS)  │
│           │    │  + Clerk JWT injection  │    │                  │
└──────────┘    └────────────────────────┘    └──────────────────┘
```

**How it works:**

1. Browser calls `/api/proxy/v1/wallets` (or any `/api/proxy/*` path)
2. Next.js server-side route handler:
   - Strips the `/api/proxy` prefix
   - Fetches the user's Clerk JWT
   - Forwards the request to `WALLET_API_BASE_URL` with `Authorization: Bearer <jwt>`
3. The enclave backend validates the JWT and returns the response
4. The proxy passes the response back to the browser

**Security properties:**

- `WALLET_API_BASE_URL` is a server-side environment variable, never exposed to the browser
- The Clerk JWT is injected server-side, never stored in browser-accessible storage
- In production, the proxy enforces TLS certificate validation

---

## Key UI Components

| Component | File | Purpose |
|:----------|:-----|:--------|
| `WalletsHub` | `src/app/wallets/WalletsHub.tsx` | Main wallet dashboard with switcher |
| `SimpleWalletDashboard` | `src/app/wallets/SimpleWalletDashboard.tsx` | Simplified single-wallet view |
| `SendForm` | `src/app/wallets/[wallet_id]/send/SendForm.tsx` | Transaction send form with gas estimation |
| `TransactionList` | `src/app/wallets/[wallet_id]/transactions/TransactionList.tsx` | Transaction history with status polling |
| `FiatRequestPanel` | `src/app/wallets/[wallet_id]/fiat/FiatRequestPanel.tsx` | Fiat on-ramp / off-ramp UI |
| `PaymentRequestBuilder` | `src/app/wallets/[wallet_id]/receive/PaymentRequestBuilder.tsx` | Payment link creation |
| `AdminPanel` | `src/app/admin/AdminPanel.tsx` | Admin dashboard components |
| `WalletBalance` | `src/components/WalletBalance.tsx` | Balance display with auto-refresh |
| `AddressQRCode` | `src/components/AddressQRCode.tsx` | QR code for wallet address |
| `WalletSwitcher` | `src/components/WalletSwitcher.tsx` | Multi-wallet selector |

---

## TypeScript API Types

API types are auto-generated from the OpenAPI specification:

```bash
# Regenerate types from openapi.json
pnpm generate-types
```

This reads `apps/wallet-web/openapi.json` and produces typed request/response interfaces in `src/types/api.ts` using `openapi-typescript`.

---

## Build for Production

```bash
pnpm build    # Production build
pnpm start    # Start production server
```

Production requirements:
- Valid `WALLET_API_BASE_URL` pointing to the enclave backend
- Proper TLS certificates (no `NODE_TLS_REJECT_UNAUTHORIZED=0`)
- Clerk production keys (replace `pk_test_` / `sk_test_` with production keys)

---

## Linting

```bash
pnpm lint     # ESLint 9 + TypeScript checks
```
