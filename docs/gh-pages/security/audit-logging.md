---
layout: default
title: Audit Logging
parent: Security
nav_order: 3
---

# Audit Logging
{: .fs-7 }

Every security-relevant operation is recorded to an append-only structured log sealed inside the SGX enclave's encrypted filesystem.
{: .fs-5 .fw-300 }

---

## Log Location and Format

Audit events are stored as newline-delimited JSON (JSONL) files:

```
/data/audit/{YYYY-MM-DD}/events.jsonl
```

Each file contains all events for that calendar day. Files are append-only --- no event is ever modified or deleted.

---

## Event Schema

```json
{
  "event_id": "evt_01HXYZ...",
  "timestamp": "2026-03-15T10:30:00.123Z",
  "event_type": "transaction_signed",
  "success": true,
  "user_id": "user_2abc123",
  "resource_type": "wallet",
  "resource_id": "wal_a1b2c3d4",
  "details": "Signed transfer of 1.5 AVAX to 0x1234...",
  "error": null,
  "ip_address": "192.168.1.1"
}
```

| Field | Type | Description |
|:------|:-----|:------------|
| `event_id` | string | Unique event identifier |
| `timestamp` | ISO 8601 | UTC timestamp with millisecond precision |
| `event_type` | enum | Event category (see table below) |
| `success` | boolean | Whether the operation succeeded |
| `user_id` | string | Authenticated user who triggered the event |
| `resource_type` | string | Type of resource affected (wallet, bookmark, etc.) |
| `resource_id` | string | ID of the affected resource |
| `details` | string | Human-readable description |
| `error` | string \| null | Error message (present only when `success: false`) |
| `ip_address` | string | Client IP address (forwarded by proxy) |

---

## Logged Event Types

### Wallet Events

| Event Type | Trigger |
|:-----------|:--------|
| `wallet_created` | `POST /v1/wallets` succeeds |
| `wallet_deleted` | `DELETE /v1/wallets/{id}` succeeds |
| `wallet_accessed` | `GET /v1/wallets/{id}` succeeds |

### Transaction Events

| Event Type | Trigger |
|:-----------|:--------|
| `transaction_signed` | secp256k1 signing completes inside enclave |
| `transaction_broadcast` | Signed tx sent to Avalanche RPC |

### Address Book Events

| Event Type | Trigger |
|:-----------|:--------|
| `bookmark_created` | `POST /v1/bookmarks` succeeds |
| `bookmark_deleted` | `DELETE /v1/bookmarks/{id}` succeeds |

### Authentication Events

| Event Type | Trigger |
|:-----------|:--------|
| `auth_success` | Valid JWT accepted |
| `auth_failure` | JWT rejected (expired, invalid signature, etc.) |
| `permission_denied` | Ownership check or role check fails |

### Administrative Events

| Event Type | Trigger |
|:-----------|:--------|
| `admin_access` | Any `/v1/admin/*` endpoint accessed |
| `config_changed` | Configuration modification |

### Fiat Events

| Event Type | Trigger |
|:-----------|:--------|
| `fiat_on_ramp_requested` | `POST /v1/fiat/onramp/requests` succeeds |
| `fiat_off_ramp_requested` | `POST /v1/fiat/offramp/requests` succeeds |

---

## Querying Audit Logs

Admins query logs via the REST API:

```http
GET /v1/admin/audit/events
Authorization: Bearer <admin-jwt>
```

### Filter Parameters

| Parameter | Example | Effect |
|:----------|:--------|:-------|
| `start_date` | `2026-03-01` | Events on or after this date |
| `end_date` | `2026-03-31` | Events on or before this date |
| `user_id` | `user_2abc123` | Events for a specific user |
| `event_type` | `transaction_signed` | Events of a specific type |
| `resource_type` | `wallet` | Events for a resource type |
| `resource_id` | `wal_a1b2c3d4` | Events for a specific resource |
| `limit` | `100` | Max results (default: 100) |
| `offset` | `0` | Pagination offset |

### Example Queries

```bash
# All transaction signing events today
curl -k "https://localhost:8080/v1/admin/audit/events?event_type=transaction_signed" \
  -H "Authorization: Bearer $ADMIN_JWT"

# Failed authentication attempts in the last week
curl -k "https://localhost:8080/v1/admin/audit/events?event_type=auth_failure&start_date=2026-03-08" \
  -H "Authorization: Bearer $ADMIN_JWT"

# Complete activity for a specific user
curl -k "https://localhost:8080/v1/admin/audit/events?user_id=user_2abc123&limit=200" \
  -H "Authorization: Bearer $ADMIN_JWT"

# All admin actions
curl -k "https://localhost:8080/v1/admin/audit/events?event_type=admin_access" \
  -H "Authorization: Bearer $ADMIN_JWT"
```

---

## Security Properties

| Property | Implementation |
|:---------|:---------------|
| **Append-only** | Log files are never modified; new events are appended only |
| **Sealed storage** | Log files are inside Gramine's encrypted FS, unreadable outside enclave |
| **Tamper evidence** | Any modification of log files would require enclave access |
| **Completeness** | Every handler that performs a sensitive operation calls the audit logger |
| **Failure logging** | Both successful and failed operations are logged (with `success: false`) |

---

## Log Retention

Audit logs accumulate in `/data/audit/` indefinitely. There is no automatic rotation or deletion in v1. Operators should:

1. Monitor total storage usage via `GET /v1/admin/health`
2. Plan for log archival based on regulatory requirements
3. Note that logs cannot be read outside the enclave --- backup requires enclave access
