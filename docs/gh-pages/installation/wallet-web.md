---
layout: default
title: Wallet Web
parent: Installation
nav_order: 2
---

# Wallet Web Installation

`wallet-web` is a Next.js 16 frontend using Clerk auth and a server-side proxy to the enclave backend.

## Prerequisites

- Node.js 20+
- `pnpm`
- Clerk application (publishable + secret keys)
- Running rust-server backend (`https://localhost:8080` by default)

## Setup

```bash
cd apps/wallet-web
pnpm install
```

Create `apps/wallet-web/.env.local` manually with values like:

```env
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=pk_test_...
CLERK_SECRET_KEY=sk_test_...
WALLET_API_BASE_URL=https://localhost:8080
NODE_TLS_REJECT_UNAUTHORIZED=0
```

Optional Clerk route overrides (if needed):

```env
NEXT_PUBLIC_CLERK_SIGN_IN_URL=/sign-in
NEXT_PUBLIC_CLERK_SIGN_UP_URL=/sign-up
```

## Run

```bash
pnpm dev
```

Open `http://localhost:3000`.

## Current Key Routes

- `/wallets`
- `/wallets/[wallet_id]/fiat`
- `/wallets/bootstrap`
- `/pay`
- `/callback`
- `/account`

## Proxy Architecture

Browser calls are sent to:

- `/api/proxy/*`

The proxy forwards to `WALLET_API_BASE_URL` and injects a Clerk JWT.

## Production Note

The proxy rejects `NODE_TLS_REJECT_UNAUTHORIZED=0` when `NODE_ENV=production`.
Use proper CA-trusted certificates in production.
