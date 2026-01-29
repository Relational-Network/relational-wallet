---
layout: default
title: Wallet Web
parent: Installation
nav_order: 2
---

# Wallet Web Installation

The wallet frontend is a Next.js 16 application with Clerk authentication.

## Prerequisites

- Node.js 18+ (LTS recommended)
- pnpm (or npm/yarn)
- A [Clerk](https://clerk.dev) account for authentication

## Local Development Setup

### 1. Install Dependencies

```bash
cd apps/wallet-web
pnpm install
```

### 2. Configure Environment

Copy the example environment file:

```bash
cp .env.local.example .env.local
```

Edit `.env.local` with your values:

```env
# Clerk Authentication
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=pk_test_...
CLERK_SECRET_KEY=sk_test_...
NEXT_PUBLIC_CLERK_SIGN_IN_URL=/sign-in
NEXT_PUBLIC_CLERK_SIGN_UP_URL=/sign-up

# Backend API (server-side only - not exposed to browser)
WALLET_API_BASE_URL=https://localhost:8080

# Development: Accept self-signed certificates from enclave
NODE_TLS_REJECT_UNAUTHORIZED=0
```

### 3. Generate TypeScript Types (Optional)

If the OpenAPI spec changes, regenerate types:

```bash
pnpm generate-types
```

### 4. Start Development Server

```bash
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

## Architecture: API Proxy

The frontend uses a server-side proxy pattern to communicate with the SGX enclave:

```
Browser → /api/proxy/* → Next.js Server → SGX Enclave
```

**Why a proxy?**

1. Browsers reject self-signed RA-TLS certificates
2. Node.js can accept them via `NODE_TLS_REJECT_UNAUTHORIZED=0`
3. The proxy adds the Clerk JWT token server-side

The proxy route is at `src/app/api/proxy/[...path]/route.ts`.

## Production Build

### Build for Production

```bash
pnpm build
```

### Start Production Server

```bash
pnpm start
```

## Deployment Checklist

Before deploying to production:

- [ ] Set `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` to production key
- [ ] Set `CLERK_SECRET_KEY` to production secret
- [ ] Set `WALLET_API_BASE_URL` to production enclave URL
- [ ] Remove `NODE_TLS_REJECT_UNAUTHORIZED=0` (use proper certificates)
- [ ] Configure HTTPS for the Next.js server
- [ ] Set up proper CSP headers
- [ ] Configure Clerk redirect URLs for production domain

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` | Yes | Clerk frontend API key |
| `CLERK_SECRET_KEY` | Yes | Clerk backend secret |
| `WALLET_API_BASE_URL` | Yes | Enclave backend URL |
| `NODE_TLS_REJECT_UNAUTHORIZED` | Dev only | Set to `0` for self-signed certs |

## Troubleshooting

### `ERR_CERT_AUTHORITY_INVALID`

Browser is making direct calls to the enclave instead of using the proxy. Make sure:
- API calls go through `/api/proxy/*`
- The `apiClient` uses the proxy for browser-side requests

### `ECONNREFUSED :8080`

The enclave backend is not running. Start it with:

```bash
cd apps/rust-server
gramine-sgx rust-server
```

### 401 Unauthorized

- Check Clerk keys are correct
- Verify the enclave has the correct `CLERK_JWKS_URL`
- Make sure you're signed in via Clerk
