---
layout: default
title: Fiat On/Off-Ramp
parent: User Guides
nav_order: 3
---

# Fiat On/Off-Ramp
{: .fs-7 }

Deposit euros via bank payment to receive rEUR tokens (on-ramp), or burn rEUR to receive euros in your bank account (off-ramp). Powered by TrueLayer.
{: .fs-5 .fw-300 }

---

## Prerequisites

- Active wallet with an Avalanche C-Chain address
- TrueLayer sandbox configured in the backend (set via environment variables)
- AVAX in your wallet for gas fees during settlement

---

## On-Ramp: EUR → rEUR

Deposit real euros (or test euros in sandbox) and receive an equivalent amount of rEUR tokens in your wallet.

### Flow Overview

```
1. Create on-ramp request
   └→ Backend creates TrueLayer payment
   └→ Returns hosted payment page URL

2. Complete bank payment on TrueLayer
   └→ TrueLayer processes your bank transfer

3. Automatic settlement (a few seconds to minutes)
   └→ Backend detects confirmed payment
   └→ Reserve wallet mints rEUR to your wallet
   └→ Request marked "completed"
```

### Via the Web UI

1. Navigate to `/wallets/[wallet_id]/fiat`
2. Select **On-Ramp**
3. Enter the EUR amount
4. Click **Continue**
5. Complete payment on the TrueLayer hosted page
6. Return to the wallet dashboard
7. Your rEUR balance updates automatically

### Via the API

**Step 1: Create the request**

```bash
curl -k -X POST https://localhost:8080/v1/fiat/onramp/requests \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_id": "wal_a1b2c3d4",
    "amount_eur": 50.00,
    "note": "Monthly top-up"
  }'
```

```json
{
  "request_id": "fiat_req_123",
  "status": "awaiting_user_deposit",
  "provider_action_url": "https://payment.truelayer-sandbox.com/payments/pay_abc",
  "amount_eur": 50.00
}
```

**Step 2: Redirect user to provider**

Open `provider_action_url` in the browser to complete the bank payment.

**Step 3: Poll for completion**

```bash
curl -k "https://localhost:8080/v1/fiat/requests/fiat_req_123" \
  -H "Authorization: Bearer $JWT"
```

Poll until `status` is `"completed"` or `"failed"`:

| Status | Meaning |
|:-------|:--------|
| `awaiting_user_deposit` | User has not completed payment yet |
| `provider_pending` | TrueLayer is processing payment |
| `settlement_pending` | rEUR minting in progress |
| `completed` | rEUR deposited to your wallet |
| `failed` | Payment failed (see `failure_reason`) |

**Step 4: Verify rEUR balance**

```bash
curl -k "https://localhost:8080/v1/wallets/wal_a1b2c3d4/balance" \
  -H "Authorization: Bearer $JWT"
# token_balances[rEUR].balance_formatted should reflect the deposit
```

---

## Off-Ramp: rEUR → EUR

Burn rEUR tokens from your wallet and receive an equivalent euro payout to your bank account.

### Prerequisites

- rEUR balance in your wallet (at least the amount you want to withdraw)
- Valid IBAN bank account

### Via the Web UI

1. Navigate to `/wallets/[wallet_id]/fiat`
2. Select **Off-Ramp**
3. Enter the EUR amount and your IBAN
4. Review and confirm
5. Monitor the request status

### Via the API

```bash
curl -k -X POST https://localhost:8080/v1/fiat/offramp/requests \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "wallet_id": "wal_a1b2c3d4",
    "amount_eur": 25.00,
    "beneficiary_account_holder_name": "John Doe",
    "beneficiary_iban": "DE89370400440532013000"
  }'
```

```json
{
  "request_id": "fiat_req_456",
  "status": "queued",
  "direction": "off_ramp",
  "amount_eur": 25.00
}
```

Off-ramp status progression:

| Status | Meaning |
|:-------|:--------|
| `queued` | Request received, processing queued |
| `settlement_pending` | rEUR burned, payout initiated |
| `provider_pending` | TrueLayer processing bank transfer |
| `completed` | Euros sent to your bank account |
| `failed` | Transfer failed (see `failure_reason`) |

---

## Listing Fiat Requests

```bash
# All requests for a wallet
curl -k "https://localhost:8080/v1/fiat/requests?wallet_id=wal_a1b2c3d4" \
  -H "Authorization: Bearer $JWT"

# Active requests only (not completed/failed)
curl -k "https://localhost:8080/v1/fiat/requests?wallet_id=wal_a1b2c3d4&active_only=true" \
  -H "Authorization: Bearer $JWT"
```

---

## Settlement Details

When an on-ramp request reaches `completed`:

- `deposit_tx_hash` — The Avalanche transaction hash where rEUR was minted to your wallet
- `reserve_transfer_tx_hash` — The transaction where the reserve wallet transferred rEUR
- View on Snowtrace: `https://testnet.snowtrace.io/tx/{deposit_tx_hash}`

---

## Sandbox Behavior

The current integration uses **TrueLayer sandbox**:

- No real money is transferred
- Use TrueLayer sandbox test bank accounts to complete payments
- Typical sandbox confirmation time: a few seconds to 1 minute
- Webhooks may be delayed; the backend poller ensures eventual completion

For production deployment, replace sandbox credentials with production TrueLayer keys.

---

## Troubleshooting

| Symptom | Likely Cause | Fix |
|:--------|:-------------|:----|
| Request stuck at `awaiting_user_deposit` | Payment not completed on TrueLayer | Visit `provider_action_url` and complete payment |
| Request stuck at `provider_pending` | TrueLayer processing delay | Wait or use `POST /v1/admin/fiat/requests/{id}/sync` |
| `failed` status with no reason | Backend error | Check admin audit logs for details |
| rEUR balance not updated after `completed` | Display cache | Refresh balance via API or UI |
| `503 Service Unavailable` | TrueLayer API down | Wait and retry; the poller will resume |
