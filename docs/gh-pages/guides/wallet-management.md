---
layout: default
title: Wallet Management
parent: User Guides
nav_order: 1
---

# Wallet Management
{: .fs-7 }

Create and manage wallets backed by secp256k1 keys generated inside the SGX enclave.
{: .fs-5 .fw-300 }

---

## Creating a Wallet

### Via the Web UI

1. Sign in at `http://localhost:3000`
2. Navigate to `/wallets`
3. Click **New Wallet**
4. Enter an optional label (e.g., "Savings", "Trading")
5. Click **Create**

The wallet is created with a new secp256k1 key pair generated inside the SGX enclave. You receive the Ethereum-compatible address immediately.

### Via the API

```bash
curl -k -X POST https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"label": "My First Wallet"}'
```

```json
{
  "wallet": {
    "wallet_id": "wal_a1b2c3d4",
    "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
    "label": "My First Wallet",
    "status": "active",
    "created_at": "2026-03-15T10:30:00Z"
  },
  "message": "Wallet created successfully"
}
```

Save the `wallet_id` and `public_address`. The `public_address` is your Avalanche C-Chain address — share it to receive AVAX and tokens.

---

## Listing Your Wallets

### Via the Web UI

Navigate to `http://localhost:3000/wallets` to see all your wallets with their balances.

### Via the API

```bash
curl -k https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT"
```

---

## Funding Your Wallet

On Fuji testnet, get free AVAX from the official faucet:

1. Go to [faucet.avax.network](https://faucet.avax.network/)
2. Select **Fuji (C-Chain)**
3. Paste your wallet's `public_address`
4. Request 2 AVAX (standard faucet amount)
5. Wait ~5 seconds for confirmation

Check your balance:

```bash
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/balance" \
  -H "Authorization: Bearer $JWT"
```

---

## Managing Your Address Book (Bookmarks)

Bookmarks are address book entries scoped to a wallet. They support both address-based and email-based contacts.

### Add an Address Contact

```bash
curl -k -X POST https://localhost:8080/v1/bookmarks \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_id": "wal_a1b2c3d4",
    "name": "Alice",
    "recipient_type": "address",
    "address": "0x1234567890abcdef1234567890abcdef12345678"
  }'
```

### Add an Email Contact

Email contacts use a SHA-256 hash of the email address (privacy-preserving):

```bash
# Compute the email hash (SHA-256 of lowercase email)
EMAIL_HASH=$(echo -n "alice@example.com" | sha256sum | cut -d' ' -f1)

curl -k -X POST https://localhost:8080/v1/bookmarks \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"wallet_id\": \"wal_a1b2c3d4\",
    \"name\": \"Alice\",
    \"recipient_type\": \"email\",
    \"email_hash\": \"$EMAIL_HASH\",
    \"email_display\": \"a***@example.com\"
  }"
```

### List Bookmarks

```bash
curl -k "https://localhost:8080/v1/bookmarks?wallet_id=wal_a1b2c3d4" \
  -H "Authorization: Bearer $JWT"
```

### Delete a Bookmark

```bash
curl -k -X DELETE "https://localhost:8080/v1/bookmarks/bkm_xyz789" \
  -H "Authorization: Bearer $JWT"
```

---

## Deleting a Wallet

Wallets are soft-deleted — your data is retained but the wallet cannot be used.

### Via the API

```bash
curl -k -X DELETE "https://localhost:8080/v1/wallets/wal_a1b2c3d4" \
  -H "Authorization: Bearer $JWT"
```

Deleted wallets:
- No longer appear in `GET /v1/wallets`
- Cannot send transactions
- Cannot receive via fiat flows
- Return `404` on direct access

---

## Wallet Statuses

| Status | What it means | What you can do |
|:-------|:--------------|:----------------|
| `active` | Normal | All operations |
| `suspended` | Suspended by admin | View only — no sends, no fiat |
| `deleted` | You deleted it | Nothing (404) |

---

## Sharing Your Address

Every wallet has an Ethereum-compatible address. Use it to receive:

- **AVAX** (native gas token)
- **rEUR** (Relational Euro ERC-20 token)
- **Any ERC-20 token** on Avalanche C-Chain

The web UI provides a QR code and click-to-copy for your address at `/wallets/[wallet_id]`.
