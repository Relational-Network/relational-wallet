---
layout: default
title: Sending & Receiving
parent: User Guides
nav_order: 2
---

# Sending & Receiving
{: .fs-7 }

Transfer AVAX and rEUR to addresses or email recipients, generate payment request links, and track your transaction history.
{: .fs-5 .fw-300 }

---

## Checking Your Balance

### Web UI

Navigate to `/wallets/[wallet_id]` to see:
- Native AVAX balance
- rEUR token balance
- Other ERC-20 balances

Click **Refresh** to update balances from the chain.

### API

```bash
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/balance" \
  -H "Authorization: Bearer $JWT"
```

```json
{
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
  "network": "fuji",
  "chain_id": 43113,
  "native_balance": {
    "symbol": "AVAX",
    "balance_formatted": "1.5",
    "decimals": 18
  },
  "token_balances": [
    {
      "symbol": "rEUR",
      "balance_formatted": "10.0",
      "decimals": 6,
      "contract_address": "0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63"
    }
  ]
}
```

---

## Sending AVAX

### Web UI

1. Navigate to `/wallets/[wallet_id]/send`
2. Enter the recipient address (or select from bookmarks)
3. Enter the amount
4. Click **Estimate Gas** to preview fees
5. Review and click **Send**
6. Wait for confirmation (typically 1-2 seconds on Fuji)

### API — Estimate First

Always estimate gas before sending:

```bash
curl -k -X POST "https://localhost:8080/v1/wallets/wal_a1b2c3d4/estimate" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "0.1",
    "to": "0x1234567890abcdef1234567890abcdef12345678",
    "network": "fuji",
    "token": "AVAX"
  }'
```

```json
{
  "gas_limit": "21000",
  "max_fee_per_gas": "30000000000",
  "estimated_cost": "0.00063"
}
```

### API — Send

```bash
curl -k -X POST "https://localhost:8080/v1/wallets/wal_a1b2c3d4/send" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "0.1",
    "to": "0x1234567890abcdef1234567890abcdef12345678",
    "network": "fuji",
    "token": "AVAX"
  }'
```

```json
{
  "tx_hash": "0xabc123def456...",
  "status": "pending",
  "explorer_url": "https://testnet.snowtrace.io/tx/0xabc123..."
}
```

---

## Sending rEUR (ERC-20)

Sending rEUR works the same way, but you need AVAX for gas fees even when sending rEUR.

```bash
curl -k -X POST "https://localhost:8080/v1/wallets/wal_a1b2c3d4/send" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "amount": "5.0",
    "to": "0x1234567890abcdef1234567890abcdef12345678",
    "network": "fuji",
    "token": "rEUR"
  }'
```

You always need AVAX in your wallet to pay for gas, even when sending ERC-20 tokens.
{: .note }

---

## Sending to an Email Address

Send to a recipient identified by their email address (resolved to their wallet address on-chain):

```bash
# Hash the recipient's email (SHA-256 of lowercase)
EMAIL_HASH=$(echo -n "alice@example.com" | sha256sum | cut -d' ' -f1)

curl -k -X POST "https://localhost:8080/v1/wallets/wal_a1b2c3d4/send" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d "{
    \"amount\": \"0.1\",
    \"to_email_hash\": \"$EMAIL_HASH\",
    \"network\": \"fuji\",
    \"token\": \"AVAX\"
  }"
```

The recipient must have a Relational Wallet linked to that email address. If not found, the request fails with `422 Unprocessable Entity`.

---

## Tracking a Transaction

### Web UI

Navigate to `/wallets/[wallet_id]/transactions` to see your full history with live status updates.

### API — Poll for Confirmation

```bash
TX_HASH="0xabc123def456..."

curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/transactions/$TX_HASH" \
  -H "Authorization: Bearer $JWT"
```

```json
{
  "tx_hash": "0xabc123def456...",
  "status": "confirmed",
  "block_number": 12345678,
  "confirmations": 5,
  "gas_used": "21000",
  "timestamp": "2026-03-15T10:35:00Z"
}
```

Poll until `status` is `"confirmed"` or `"failed"`. Transactions typically confirm in under 5 seconds on Fuji.

### View on Snowtrace

Every transaction response includes an `explorer_url`:

```
https://testnet.snowtrace.io/tx/0xabc123def456...
```

---

## Creating a Payment Link

Request payment from anyone with a shareable link — no sign-in required to view.

### API

```bash
curl -k -X POST "https://localhost:8080/v1/wallets/wal_a1b2c3d4/payment-link" \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "recipient_type": "address",
    "amount": "0.5",
    "token": "AVAX",
    "expires_hours": 24,
    "note": "Split dinner",
    "single_use": false
  }'
```

```json
{
  "token": "pay_xyz789abc",
  "expires_at": "2026-03-16T10:30:00Z"
}
```

Share the link: `http://localhost:3000/pay?token=pay_xyz789abc`

### Resolving a Payment Link (No Auth Required)

```bash
curl -k "https://localhost:8080/v1/payment-link/pay_xyz789abc"
```

```json
{
  "recipient_type": "address",
  "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
  "amount": "0.5",
  "note": "Split dinner",
  "token_type": "AVAX"
}
```

---

## Transaction History

### API — Paginated History

```bash
# First page
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/transactions?limit=10" \
  -H "Authorization: Bearer $JWT"

# Next page using cursor from response
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/transactions?limit=10&cursor=cursor_xyz" \
  -H "Authorization: Bearer $JWT"

# Filter by direction
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/transactions?direction=sent" \
  -H "Authorization: Bearer $JWT"
```
