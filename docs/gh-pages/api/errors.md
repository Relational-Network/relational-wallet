---
layout: default
title: Errors
parent: API Reference
nav_order: 6
---

# Error Reference
{: .fs-7 }

All API errors follow a consistent JSON structure. Use the `error` field to identify the error type programmatically and `message` for human-readable context.
{: .fs-5 .fw-300 }

---

## Error Response Format

```json
{
  "error": "not_found",
  "message": "Wallet not found",
  "request_id": "req_abc123"
}
```

| Field | Type | Description |
|:------|:-----|:------------|
| `error` | string | Machine-readable error code |
| `message` | string | Human-readable description |
| `request_id` | string | Tracing ID (correlates with backend logs via `x-request-id`) |

---

## HTTP Status Codes

### `400 Bad Request`

The request is malformed or contains invalid parameters.

```json
{
  "error": "bad_request",
  "message": "Invalid Ethereum address format"
}
```

Common causes:
- Invalid address format (not a valid 0x hex address)
- Missing required fields in request body
- Invalid date format in query parameters
- Amount is not a valid number

---

### `401 Unauthorized`

Authentication failed. The JWT is missing, expired, or has an invalid signature.

```json
{
  "error": "unauthorized",
  "message": "Missing authorization header"
}
```

| Sub-case | Message |
|:---------|:--------|
| No header | `Missing authorization header` |
| Invalid format | `Invalid authorization header format` |
| Expired token | `Token expired` |
| Invalid signature | `Invalid token signature` |
| Invalid issuer | `Invalid token issuer` |
| JWKS fetch failure | `Failed to verify token` |

**Resolution:** Obtain a fresh token from Clerk. Tokens expire after the configured session duration.

---

### `403 Forbidden`

The user is authenticated but does not have permission to perform this action.

```json
{
  "error": "forbidden",
  "message": "Not authorized to access this wallet"
}
```

| Sub-case | Message |
|:---------|:--------|
| Wrong owner | `Not authorized to access this wallet` |
| Insufficient role | `Admin access required` |
| Suspended wallet | `Wallet is suspended` |

---

### `404 Not Found`

The requested resource does not exist or has been deleted.

```json
{
  "error": "not_found",
  "message": "Wallet not found"
}
```

Note: deleted wallets return `404`, not the wallet with `status: "deleted"`. Ownership violations also return `404` on some endpoints to avoid leaking resource existence.

---

### `422 Unprocessable Entity`

The request is well-formed but cannot be executed due to business logic constraints.

```json
{
  "error": "unprocessable_entity",
  "message": "Insufficient balance: have 0.001 AVAX, need 0.1 AVAX + gas"
}
```

Common causes:
- Insufficient AVAX or token balance
- Sending to an unresolvable email hash

---

### `503 Service Unavailable`

A required external dependency is unavailable.

```json
{
  "error": "service_unavailable",
  "message": "RPC node unavailable"
}
```

Common causes:
- Avalanche RPC node is down or rate-limiting
- TrueLayer API unreachable

The backend will continue to serve non-blockchain requests during an RPC outage.

---

## Request Tracing

Every response includes an `x-request-id` header. Include this ID when reporting issues:

```bash
curl -v -k https://localhost:8080/v1/wallets \
  -H "Authorization: Bearer $JWT" 2>&1 | grep -i "x-request-id"
# < x-request-id: req_abc123
```

The same ID appears in the backend logs for correlation.
