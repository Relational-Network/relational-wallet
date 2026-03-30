---
layout: default
title: Threat Model
parent: Security
nav_order: 1
---

# Threat Model
{: .fs-7 }

What Relational Wallet protects against, what it relies on as trusted, and known limitations.
{: .fs-5 .fw-300 }

---

## Assets Being Protected

| Asset | Sensitivity | Location |
|:------|:------------|:---------|
| Wallet private keys | Critical | SGX enclave memory + sealed FS |
| User wallet metadata | High | Sealed FS at `/data/wallets/` |
| Transaction history | Medium | Sealed FS at `/data/transactions/` |
| Fiat request records | High | Sealed FS at `/data/fiat/` |
| Reserve wallet key | Critical | SGX enclave sealed FS |
| Audit logs | High | Sealed FS at `/data/audit/` |
| Clerk JWT tokens | High | Browser session (short-lived) |
| TrueLayer credentials | Critical | Environment variables (not persisted) |

---

## Threat Matrix

### T1 — Malicious Server Operator

**Threat:** Server operator attempts to steal private keys or user data.

**Mitigation:**
- All cryptographic material lives inside SGX enclave memory
- Gramine encrypted FS seals data to enclave identity — operator cannot decrypt
- No API endpoint to export private keys
- Audit logs record all operations and are sealed to the enclave

**Residual risk:** Low. Operator can kill the process (DoS) but cannot extract keys.

---

### T2 — Compromised Host OS / Hypervisor

**Threat:** Attacker with root access on the host OS attempts to read enclave memory or exfiltrate data.

**Mitigation:**
- SGX hardware memory encryption (TSME) prevents host OS from reading enclave pages
- Gramine encrypted FS ensures disk files are ciphertext even with root access
- RA-TLS ensures clients connect to the genuine enclave, not a fake process

**Residual risk:** Low. SGX provides hardware-level isolation from the OS.

---

### T3 — Man-in-the-Middle Attack

**Threat:** Network attacker intercepts connections between the frontend and backend, or between the backend and external services.

**Mitigation:**
- RA-TLS: TLS connection includes DCAP attestation evidence; clients verify MRENCLAVE before trusting
- Standard TLS 1.3 for all external connections (Clerk JWKS, TrueLayer, Avalanche RPC)
- The Nginx proxy provides a Let's Encrypt certificate for external webhook ingress

**Residual risk:** Low for enclave connections. External service connections rely on standard PKI.

---

### T4 — Binary Tampering / Supply Chain Attack

**Threat:** Attacker modifies the deployed binary to add backdoors or steal keys.

**Mitigation:**
- MRENCLAVE measurement changes with any code or configuration modification
- CI/CD pins MRENCLAVE in `measurements.toml` and fails if it changes unexpectedly
- Deterministic Docker builds produce reproducible measurements
- Clients can independently verify the MRENCLAVE before connecting

**Residual risk:** Medium. Requires trust in the build pipeline and enclave signing key. Signing key compromise would allow deploying a different binary with a valid MRSIGNER.

---

### T5 — Authentication Bypass

**Threat:** Attacker crafts a JWT to impersonate another user or gain admin access.

**Mitigation:**
- JWT signatures verified against Clerk's JWKS (RS256/RS384/RS512/ES256)
- JWKS cached with 2× TTL grace; fails closed after grace expires
- Issuer (`CLERK_ISSUER`) validated on every request
- Audience (`CLERK_AUDIENCE`) optionally validated (recommended in production)

**Residual risk:** Low. Relies on Clerk's key management and cryptographic signing.

---

### T6 — Horizontal Privilege Escalation

**Threat:** User accesses another user's wallets, transactions, or bookmarks.

**Mitigation:**
- Every wallet-scoped operation loads metadata and checks `meta.user_id == authenticated_user.user_id`
- Returns `403 Forbidden` on ownership mismatch
- Admin endpoints require explicit `admin` role

**Residual risk:** Very low. Ownership enforcement is applied at every handler.

---

### T7 — Fiat Flow Manipulation

**Threat:** Attacker creates fraudulent fiat requests to mint rEUR without a real deposit.

**Mitigation:**
- On-ramp settlement only proceeds after TrueLayer confirms payment
- Webhook validated with HMAC signature (when `TRUELAYER_WEBHOOK_SHARED_SECRET` is configured)
- Settlement triggers only when request status reaches `settlement_pending`
- All fiat operations are audit-logged

**Residual risk:** Medium. Currently relies on TrueLayer sandbox; production requires production TrueLayer credentials and webhook secret.

---

### T8 — Denial of Service

**Threat:** Attacker floods the API with requests, rendering the service unavailable.

**Mitigation:**
- Nginx proxy provides rate limiting on webhook endpoint (10 req/s, burst 20)
- Axum + Tokio async handling with connection limits

**Residual risk:** Medium. No rate limiting on authentication failures (known limitation). Backend rate limiting is a pending improvement.

---

## Trust Assumptions

The following components are **trusted** (outside the threat model):

| Component | Why trusted |
|:----------|:------------|
| Intel SGX hardware | Hardware root of trust; manufacturer (Intel) is trusted |
| Gramine 1.8 | Reviewed open-source library OS; version pinned |
| Clerk identity platform | Third-party auth provider; JWT cryptography is verifiable |
| Avalanche C-Chain | Blockchain consensus; transaction finality is probabilistic |
| TrueLayer | Third-party fiat provider; production requires trust in their PCI DSS compliance |

---

## Known Limitations

| Limitation | Status | Mitigation |
|:-----------|:-------|:-----------|
| No rate limiting on auth failures | Known gap | Future work: add `tower::limit` or governor middleware |
| `CLERK_AUDIENCE` optional | Warning logged | Set `CLERK_AUDIENCE` in production |
| Side-channel attacks on SGX | Out of scope | Gramine mitigations applied; complete protection not feasible |
| No key rotation mechanism | v1 limitation | Keys are sealed to enclave; rotation requires wallet migration |
| TrueLayer sandbox | Non-production | Production credentials required for live fiat flows |
