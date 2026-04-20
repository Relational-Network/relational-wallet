---
layout: default
title: Security
nav_order: 6
has_children: true
permalink: /security/
---

# Security
{: .fs-8 }

Hardware-enforced privacy, cryptographic authentication, and comprehensive audit trails.
{: .fs-5 .fw-300 }

---

## Security Architecture Overview

Relational Wallet's security model is built on three mutually reinforcing layers:

```
Layer 1: Hardware Isolation (Intel SGX)
  → Private keys never leave the enclave
  → Host OS cannot read enclave memory
  → Remote attestation proves code integrity

Layer 2: Cryptographic Authentication (Clerk JWT + DCAP RA-TLS)
  → Every API call carries a signed JWT
  → Every TLS connection verifies the enclave identity
  → Role-based access control with ownership enforcement

Layer 3: Encrypted Persistence (Gramine Sealed FS)
  → All data sealed to enclave identity at rest
  → Unreadable without the enclave, even with disk access
  → Append-only audit log for all security events
```

---

## Security Properties

| Property | Guarantee | Mechanism |
|:---------|:----------|:----------|
| **Key isolation** | Private keys cannot be extracted | SGX memory isolation + no export API |
| **Data confidentiality** | Storage unreadable outside enclave | Gramine encrypted filesystem |
| **Code integrity** | Running code matches expected binary | MRENCLAVE measurement + CI pinning |
| **Attestation** | Clients can verify server code | DCAP RA-TLS with embedded quote |
| **Authentication** | Requests tied to verified identities | Clerk JWT with JWKS verification |
| **Authorization** | Users access only their own resources | Ownership checks + role enforcement |
| **Auditability** | All actions traceable | Append-only structured audit log |
| **Non-repudiation** | On-chain transaction proof | Avalanche C-Chain immutability |

---

## Responsible Disclosure

If you discover a security vulnerability, please report it responsibly:

- **Email:** security@relational.network *(replace with actual contact)*
- **GitHub:** Use [GitHub's private vulnerability reporting](https://github.com/Relational-Network/relational-wallet/security/advisories/new)
- **Do not** open a public GitHub issue for security vulnerabilities

We aim to respond within 48 hours and resolve critical issues within 7 days.

---

## Sub-pages

- [**Threat Model**](/relational-wallet/security/threat-model) --- What we protect against, trust boundaries, and known limitations
- [**Key Management**](/relational-wallet/security/key-management) --- How secp256k1 keys are generated, stored, and used
- [**Audit Logging**](/relational-wallet/security/audit-logging) --- Security event logging, querying, and retention
