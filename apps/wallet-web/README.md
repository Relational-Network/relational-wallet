# Relational Wallet - Web Frontend

A Next.js frontend for Relational Wallet, a custodial Avalanche wallet service whose backend runs inside an Intel SGX enclave.

## Features

- **Clerk Authentication**: Sign in/sign up using Clerk's hosted UI
- **Typed API Client**: All API calls use TypeScript types generated from the backend OpenAPI spec
- **API Proxy**: Browser requests are proxied through Next.js to handle RA-TLS certificates

## Tech Stack

- **Framework**: Next.js 16 (App Router)
- **Auth**: Clerk (`@clerk/nextjs`)
- **API Typing**: `openapi-typescript`
- **HTTP**: Native `fetch`
- **State**: React local state only

## Architecture

```
┌─────────────────┐       ┌──────────────────┐       ┌────────────────────┐
│   Browser       │──────▶│   Next.js        │──────▶│   SGX Enclave      │
│                 │       │   /api/proxy/*   │       │   (RA-TLS)         │
│  React Client   │       │                  │       │                    │
│  (Clerk Auth)   │       │  Adds JWT token  │       │  https://host:8080 │
└─────────────────┘       └──────────────────┘       └────────────────────┘
```

Browser requests go through `/api/proxy/[...path]` because:
1. The backend uses RA-TLS with a self-signed certificate
2. Browsers reject self-signed certificates
3. Server-side Node.js can use `NODE_TLS_REJECT_UNAUTHORIZED=0`

## Getting Started

### 1. Install Dependencies

```bash
pnpm install
```

### 2. Configure Environment Variables

Copy the example environment file and fill in your Clerk credentials:

```bash
cp .env.local.example .env.local
```

Edit `.env.local` with your values:

```env
NEXT_PUBLIC_CLERK_PUBLISHABLE_KEY=pk_test_...
CLERK_SECRET_KEY=sk_test_...
WALLET_API_BASE_URL=https://localhost:8080
NODE_TLS_REJECT_UNAUTHORIZED=0
```

Get your Clerk keys from [dashboard.clerk.com](https://dashboard.clerk.com).

### 3. Start the Backend

In a separate terminal, start the enclave backend:

```bash
cd ../rust-server
make
gramine-sgx rust-server
```

### 4. Run the Development Server

```bash
pnpm dev
```

Open [http://localhost:3000](http://localhost:3000) to view the app.

## Project Structure

```
src/
├── app/
│   ├── api/
│   │   └── proxy/[...path]/    # API proxy to backend (handles RA-TLS certs)
│   ├── layout.tsx              # Root layout with ClerkProvider
│   ├── page.tsx                # Landing page (redirects if signed in)
│   ├── sign-in/                # Clerk sign-in page
│   ├── sign-up/                # Clerk sign-up page
│   ├── account/                # User account page
│   └── wallets/
│       ├── page.tsx            # Wallets list
│       ├── new/page.tsx        # Create wallet
│       └── [wallet_id]/        # Wallet detail with balance display
├── components/
│   ├── WalletBalance.tsx       # Wallet balance display with refresh
│   ├── WalletCard.tsx          # Single wallet display
│   └── WalletList.tsx          # Wallet list display
├── lib/
│   ├── api.ts                  # Typed API client (uses proxy in browser)
│   └── auth.ts                 # Clerk auth helpers
├── types/
│   └── api.ts                  # Generated OpenAPI types
└── middleware.ts               # Route protection
```

## Features

### Wallet Balance Display

The wallet detail page (`/wallets/[id]`) displays real-time balance from the Avalanche blockchain:

- **Native AVAX balance** from Fuji testnet
- **ERC-20 token balances** (USDC on Fuji)
- **Refresh button** to fetch latest balance
- **Network indicator** showing Fuji testnet
- **Faucet links** to get test AVAX and USDC
- **Loading and error states** for good UX

The balance is fetched via the backend's `/v1/wallets/{id}/balance` endpoint, which queries the Avalanche C-Chain RPC for both native AVAX and ERC-20 tokens.

#### Token Display

The `WalletBalance` component displays:
- Native AVAX balance with wei value
- USDC token balance with contract address
- Network details (chain ID, network name)
- Last updated timestamp

#### Faucet Links

For development/testing on Fuji testnet:
- **AVAX**: [Avalanche Faucet](https://core.app/tools/testnet-faucet/)
- **USDC**: [Circle Faucet](https://faucet.circle.com/)

## API Integration

### How It Works

1. **Browser** makes request to `/api/proxy/v1/wallets`
2. **Next.js API route** adds Clerk JWT and forwards to backend
3. **Backend** (SGX enclave) processes request
4. Response flows back through the proxy

This approach allows development with self-signed RA-TLS certificates.

### OpenAPI Types

Types are generated from `openapi.json` using `openapi-typescript`:

```bash
pnpm generate-types
```

### Using the API Client

```typescript
import { apiClient, type Wallet } from "@/lib/api";

// Get session token from Clerk
const token = await getToken();

// Make typed API call (automatically uses proxy in browser)
const response = await apiClient.listWallets(token);
if (response.success) {
  const wallets: Wallet[] = response.data;
}
```

### API Proxy

The proxy route at `/api/proxy/[...path]` handles:

- Forwarding requests to the backend (`WALLET_API_BASE_URL`)
- Adding the Clerk JWT token as `Authorization: Bearer <token>`
- Accepting self-signed certificates (via `NODE_TLS_REJECT_UNAUTHORIZED=0`)

For production, you would use properly signed certificates and may not need the proxy.

## Routes

| Route | Description | Auth Required |
|-------|-------------|---------------|
| `/` | Landing page | No |
| `/sign-in` | Clerk sign-in | No |
| `/sign-up` | Clerk sign-up | No |
| `/wallets` | List all wallets | Yes |
| `/wallets/new` | Create new wallet | Yes |
| `/wallets/[id]` | Wallet details | Yes |
| `/account` | User account info | Yes |
| `/api/proxy/*` | Backend proxy | Yes |

## Development Notes

### Running with Backend

To test against the real SGX enclave backend:

1. Start the backend in SGX:
   ```bash
   cd apps/rust-server
   gramine-sgx rust-server
   ```

2. Start the frontend:
   ```bash
   cd apps/wallet-web
   pnpm dev
   ```

3. Navigate to `http://localhost:3000`

### Error Handling

The app handles these error cases:

- **401 Unauthorized**: Redirects to sign-in
- **403 Forbidden**: Shows "Access denied" message
- **404 Not Found**: Shows not-found page
- **Network errors**: Shows generic error message with details

### Troubleshooting

| Issue | Solution |
|-------|----------|
| `ERR_CERT_AUTHORITY_INVALID` | Make sure you're using `/api/proxy/*` routes, not direct backend calls |
| `ECONNREFUSED` | Backend not running. Start with `gramine-sgx rust-server` |
| `401 Unauthorized` | Check Clerk config and JWKS URL in backend |

