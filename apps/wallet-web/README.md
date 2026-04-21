# wallet-web

Next.js 16 App Router frontend for the Relational Wallet, authenticated via Clerk. Deployed once per backend instance on Vercel — point it at the host running rust-server.

## Stack

- Next.js 16 (App Router) · React 19 · TypeScript
- `@clerk/nextjs` for auth (sign-in, JWT issuance, route protection in [`src/middleware.ts`](src/middleware.ts))
- `openapi-typescript` generates [`src/types/api.ts`](src/types/api.ts) from [`openapi.json`](openapi.json)
- Native `fetch` only — no client state library

## Architecture

```
Browser ──► /api/proxy/[...path] (Next route) ──► https://<instance>.duckdns.org
              │
              └─ injects Clerk JWT as Authorization: Bearer <token>
```

The proxy route at [`src/app/api/proxy/[...path]/`](src/app/api/proxy) exists so the browser never talks to the backend directly. In production the backend is fronted by nginx with a real Let's Encrypt cert (no proxy needed in principle), but routing through Next keeps the JWT injection server-side and lets dev work against the self-signed RA-TLS endpoint via `NODE_TLS_REJECT_UNAUTHORIZED=0`.

## Configuration

Copy [`.env.example`](.env.example) to `.env.local` (dev) or set the same vars in Vercel project settings (production):

| Variable | Purpose |
|----------|---------|
| `NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY` | Clerk client key |
| `CLERK_SECRET_KEY` | Clerk server key |
| `WALLET_API_BASE_URL` | Backend URL — e.g. `https://wallet-001.duckdns.org` (prod) or `https://localhost:8080` (dev) |
| `NEXT_PUBLIC_CLERK_SIGN_IN_URL` / `_SIGN_UP_URL` | Optional URL overrides (default: `/sign-in`, `/sign-up`) |
| `NODE_TLS_REJECT_UNAUTHORIZED` | Set to `0` only if pointing at a self-signed RA-TLS backend (dev or no-proxy production) |

After deploying to Vercel, add the resulting URL (e.g. `https://wallet-001.vercel.app`) to `CORS_ALLOWED_ORIGINS` in the rust-server env file on the host, then `sudo systemctl restart rust-server`.

## Develop

```bash
pnpm install
pnpm generate-types     # regenerate src/types/api.ts from openapi.json
pnpm dev                # http://localhost:3000
pnpm lint
pnpm build
```

To regenerate types after the backend OpenAPI spec changes, copy [`apps/rust-server/`](../rust-server)'s output to [`openapi.json`](openapi.json) and run `pnpm generate-types`.

## Routes

| Path | Auth |
|------|------|
| `/` · `/sign-in` · `/sign-up` | public |
| `/wallets`, `/wallets/new`, `/wallets/[id]`, `/wallets/[id]/{send,transactions}` | required |
| `/account` | required |
| `/api/proxy/*` | required (server-side, injects JWT) |

Route protection lives in [`src/middleware.ts`](src/middleware.ts).

## Deploy

```bash
vercel link             # one-time, ties this dir to a Vercel project
vercel --prod
```

Or push to a branch wired to Vercel's GitHub integration. Each backend instance gets its own Vercel project so `WALLET_API_BASE_URL` and Clerk keys can differ per environment.

---

SPDX-License-Identifier: AGPL-3.0-or-later · Copyright (C) 2026 Relational Network
