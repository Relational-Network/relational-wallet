---
layout: default
title: Cross-Instance Discovery
parent: Architecture
nav_order: 5
---

# Cross-Instance Discovery (Phase 2)
{: .fs-7 }

Privacy-preserving email-to-wallet resolution across federated enclave instances using VOPRF.
{: .fs-5 .fw-300 }

---

## Overview

Relational Wallet supports **cross-instance email discovery**: when a user sends funds to an email address, the system can resolve the recipient's on-chain address even if that recipient's wallet lives on a different enclave instance.

This is achieved using a **Verifiable Oblivious Pseudo-Random Function (VOPRF)** protocol that prevents any single party --- including the querying enclave --- from learning which email address is being looked up.

---

## Trust Model

### Source of Trust: RA-TLS

The security of cross-instance discovery relies entirely on **DCAP RA-TLS mutual authentication** between peer enclaves. Every peer-to-peer connection is established over TLS where:

1. **Both sides present RA-TLS certificates** containing embedded Intel SGX attestation evidence (DCAP quotes).
2. **The TLS handshake verifies** that each peer is running genuine SGX-protected code with the expected measurement (MRENCLAVE/MRSIGNER).
3. **No JWT tokens or user credentials** are involved in peer-to-peer communication.

This means:

| Property | Guarantee |
|:---------|:----------|
| **Peer authenticity** | Only genuine SGX enclaves with the correct code measurement can participate |
| **Transport confidentiality** | All peer traffic is encrypted via TLS |
| **No information leakage** | The VOPRF protocol ensures the querying enclave cannot learn which email maps to which address on the responding enclave |
| **Tamper resistance** | Gramine's encrypted filesystem protects stored VOPRF tokens and peer registry at rest |

### What RA-TLS Does NOT Protect

| Non-Threat | Reason |
|:-----------|:-------|
| **Denial of service** | A peer enclave can refuse to respond; rate limiting is enforced at the proxy layer |
| **Stale peer list** | The peer registry is loaded from encrypted storage and managed by admin API; operators must keep it current |
| **Network partitioning** | Discovery degrades gracefully --- local lookup succeeds independently of peer availability |

---

## Protocol Flow

```
┌──────────────┐                           ┌──────────────┐
│  Enclave A   │                           │  Enclave B   │
│  (querier)   │                           │  (responder) │
└──────┬───────┘                           └──────┬───────┘
       │                                          │
       │  1. Client blinds email hash             │
       │     (VOPRF blind)                        │
       │                                          │
       │  2. RA-TLS mutual auth                   │
       │  ────────────────────────────────────►    │
       │                                          │
       │  3. Send blinded element                 │
       │  ────── /internal/discovery/evaluate ──► │
       │                                          │
       │  4. Enclave B evaluates with its         │
       │     VOPRF secret key                     │
       │  ◄──────── evaluated element ────────    │
       │                                          │
       │  5. Client finalizes token               │
       │     (VOPRF finalize)                     │
       │                                          │
       │  6. Send finalized token                 │
       │  ────── /internal/discovery/lookup ────► │
       │                                          │
       │  7. Enclave B looks up token in          │
       │     its VOPRF store                      │
       │  ◄──────── public_address or null ────   │
       │                                          │
```

### Key Properties

- **Step 3**: Enclave B sees only a blinded element --- it cannot determine which email is being queried.
- **Step 7**: The finalized token is deterministic for a given (email, VOPRF key) pair, enabling O(1) lookup.
- **Self-skip**: A node's own VOPRF public key is excluded from the peer registry, preventing redundant self-queries.

---

## Peer Management

Peers are managed via the admin API (`/v1/admin/peers`) and stored in the encrypted filesystem at `/data/system/discovery_peers.json`.

| Endpoint | Method | Description |
|:---------|:-------|:------------|
| `/v1/admin/peers/self` | GET | Show this node's VOPRF public key and ID |
| `/v1/admin/peers` | GET | List all registered peers |
| `/v1/admin/peers` | POST | Add a new peer (URL + VOPRF public key) |
| `/v1/admin/peers/{id}` | PUT | Update a peer |
| `/v1/admin/peers/{id}` | DELETE | Remove a peer |

Each peer entry contains:

```json
{
  "node_id": "unique-node-identifier",
  "url": "https://peer-enclave.example.com:8080",
  "voprf_public_key": "base64-encoded-ristretto-point"
}
```

---

## Integration with Send Flow

When a user sends funds to `to_email_hash`:

1. **Local lookup first**: Check the local email→wallet index (O(1) via redb).
2. **Discovery fan-out**: If not found locally, query all registered peers via the VOPRF protocol.
3. **First match wins**: The first peer to return an address is used as the recipient.
4. **Graceful degradation**: If all peers fail or return no match, the send fails with "No wallet found for this email."

---

## Security Considerations

### Addressed by Design

- **Email privacy**: VOPRF prevents peers from learning which email is being queried.
- **Replay protection**: Each VOPRF evaluation is tied to the responder's secret key; replaying an evaluated element to a different peer yields a different token.
- **Enclave isolation**: VOPRF server keys are generated and sealed inside the enclave's encrypted filesystem.

### Operator Responsibilities

- **Peer registry accuracy**: Operators must ensure peer entries point to genuine enclave instances.
- **Rate limiting**: Production deployments should enforce rate limits at the proxy layer to prevent discovery query abuse.
- **Key rotation**: VOPRF server keys are generated once and sealed; key rotation requires re-registration of all VOPRF tokens.
