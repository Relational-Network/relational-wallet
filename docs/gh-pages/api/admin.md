---
layout: default
title: Admin
parent: API Reference
nav_order: 5
---

# Admin API
{: .fs-7 }

Administrative endpoints for system statistics, user management, wallet suspension, audit logs, and fiat reserve operations. All endpoints require the `admin` role.
{: .fs-5 .fw-300 }

---

All admin endpoints require `Authorization: Bearer <jwt>` where the JWT has `publicMetadata.role = "admin"`. Non-admin users receive `403 Forbidden`.
{: .warning }

---

## System Statistics

```http
GET /v1/admin/stats
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "total_wallets": 42,
  "active_wallets": 38,
  "suspended_wallets": 2,
  "deleted_wallets": 2,
  "total_bookmarks": 156,
  "uptime_seconds": 86400,
  "timestamp": "2026-03-15T10:30:00Z"
}
```

---

## Detailed Health

More comprehensive than the public `/health` endpoint. Includes storage metrics and configuration status.

```http
GET /v1/admin/health
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "status": "healthy",
  "storage": {
    "data_dir": "/data",
    "exists": true,
    "writable": true,
    "total_files": 245
  },
  "auth_configured": true,
  "version": "0.1.0",
  "build_time": "2026-03-10T08:00:00Z"
}
```

---

## List All Users

Returns all users who have wallets or bookmarks, with resource counts.

```http
GET /v1/admin/users
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "total": 15,
  "users": [
    {
      "user_id": "user_2abc123",
      "wallet_count": 3,
      "bookmark_count": 12
    },
    {
      "user_id": "user_2def456",
      "wallet_count": 1,
      "bookmark_count": 0
    }
  ]
}
```

---

## List All Wallets

Returns all wallets across all users.

```http
GET /v1/admin/wallets
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "total": 42,
  "wallets": [
    {
      "wallet_id": "wal_a1b2c3d4",
      "owner_user_id": "user_2abc123",
      "public_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD28",
      "status": "active",
      "created_at": "2026-03-15T10:30:00Z"
    }
  ]
}
```

---

## Suspend Wallet

Suspend a wallet by ID. The wallet owner cannot perform operations on suspended wallets until reactivated.

```http
POST /v1/admin/wallets/{wallet_id}/suspend
Authorization: Bearer <jwt>
```

### Response `200 OK`

Empty body. Wallet status changed to `suspended`.

### Example

```bash
curl -k -X POST https://localhost:8080/v1/admin/wallets/wal_a1b2c3d4/suspend \
  -H "Authorization: Bearer $JWT"
```

---

## Activate Wallet

Reactivate a previously suspended wallet.

```http
POST /v1/admin/wallets/{wallet_id}/activate
Authorization: Bearer <jwt>
```

### Response `200 OK`

Empty body. Wallet status changed to `active`.

---

## Query Audit Logs

Search and filter security audit events. Supports date range, user, event type, and resource filtering.

```http
GET /v1/admin/audit/events
Authorization: Bearer <jwt>
```

### Query Parameters

| Parameter | Type | Required | Description |
|:----------|:-----|:---------|:------------|
| `start_date` | string | No | Start date (YYYY-MM-DD) |
| `end_date` | string | No | End date (YYYY-MM-DD) |
| `user_id` | string | No | Filter by user ID |
| `event_type` | string | No | Filter by event type |
| `resource_type` | string | No | Filter by resource type |
| `resource_id` | string | No | Filter by resource ID |
| `limit` | integer | No | Max results (default: 100) |
| `offset` | integer | No | Pagination offset |

### Event Types

| Event Type | Description |
|:-----------|:------------|
| `wallet_created` | New wallet generated |
| `wallet_deleted` | Wallet soft-deleted |
| `wallet_accessed` | Wallet metadata read |
| `transaction_signed` | Transaction signed inside enclave |
| `transaction_broadcast` | Transaction sent to chain |
| `bookmark_created` | Bookmark added |
| `bookmark_deleted` | Bookmark removed |
| `auth_success` | Successful authentication |
| `auth_failure` | Failed authentication attempt |
| `permission_denied` | Unauthorized access attempt |
| `admin_access` | Admin endpoint accessed |
| `config_changed` | Configuration modification |
| `fiat_on_ramp_requested` | Fiat deposit initiated |
| `fiat_off_ramp_requested` | Fiat withdrawal initiated |

### Example

```bash
# Recent transaction signing events
curl -k "https://localhost:8080/v1/admin/audit/events?event_type=transaction_signed&limit=20" \
  -H "Authorization: Bearer $JWT"

# All events for a specific user in March 2026
curl -k "https://localhost:8080/v1/admin/audit/events?user_id=user_2abc123&start_date=2026-03-01&end_date=2026-03-31" \
  -H "Authorization: Bearer $JWT"

# Failed authentication attempts
curl -k "https://localhost:8080/v1/admin/audit/events?event_type=auth_failure&limit=50" \
  -H "Authorization: Bearer $JWT"
```

### Response `200 OK`

```json
{
  "events": [
    {
      "event_id": "evt_abc123",
      "timestamp": "2026-03-15T10:30:00Z",
      "event_type": "transaction_signed",
      "success": true,
      "user_id": "user_2abc123",
      "resource_type": "wallet",
      "resource_id": "wal_a1b2c3d4",
      "details": "Signed transfer of 1.5 AVAX to 0x1234...",
      "ip_address": "192.168.1.1"
    }
  ],
  "total": 1,
  "has_more": false
}
```

---

## Reserve Wallet Status

Get the fiat reserve (service) wallet status, including AVAX and rEUR balances.

```http
GET /v1/admin/fiat/service-wallet
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "wallet_id": "service_wal_001",
  "public_address": "0xReserveWalletAddress...",
  "bootstrapped": true,
  "chain_network": "fuji",
  "reur_contract_address": "0x76568BEd5Acf1A5Cd888773C8cAe9ea2a9131A63",
  "avax_balance": "10.5",
  "reur_balance": "5000.0",
  "reur_balance_raw": "5000000000"
}
```

---

## Manual Fiat Sync

Force-sync a fiat request's status with TrueLayer. Useful when webhooks are delayed or missed.

```http
POST /v1/admin/fiat/requests/{request_id}/sync
Authorization: Bearer <jwt>
```

### Response `200 OK`

```json
{
  "request": {
    "request_id": "fiat_req_123",
    "status": "completed",
    "updated_at": "2026-03-15T10:35:00Z"
  }
}
```
