---
layout: default
title: Key Management
parent: Security
nav_order: 2
---

# Key Management
{: .fs-7 }

How wallet private keys are generated, stored, and used entirely within the SGX enclave boundary.
{: .fs-5 .fw-300 }

---

## Key Types

| Key | Algorithm | Purpose | Location |
|:----|:----------|:--------|:---------|
| **Wallet key** | secp256k1 | Sign Avalanche C-Chain transactions | `/data/wallets/{id}/key.pem` (sealed) |
| **Reserve wallet key** | secp256k1 | Mint/burn rEUR for fiat settlement | `/data/system/fiat_service_wallet/key.pem` (sealed) |
| **Enclave signing key** | RSA 3072-bit | Sign SGX SIGSTRUCT (identifies enclave) | Host filesystem (operator-controlled) |
| **Storage encryption key** | AES (Gramine) | Encrypt all `/data` files | Derived from enclave identity, never stored |
| **RA-TLS key** | RSA/EC (ephemeral) | TLS connection key embedded in attestation cert | Generated at enclave startup, in-memory only |

---

## Wallet Key Lifecycle

### 1. Generation

When `POST /v1/wallets` is called:

```
Inside SGX enclave:
  │
  ▼
k256 crate generates secp256k1 key pair
  │
  ▼
Public key → Ethereum address (keccak256 of uncompressed pubkey, last 20 bytes)
  │
  ▼
Private key serialized to PEM format
  │
  ▼
PEM written to /data/wallets/{wallet_id}/key.pem
  (Gramine encrypts before writing to host disk)
  │
  ▼
Public address returned to caller
Private key NEVER leaves the enclave
```

### 2. Storage

```
/data/wallets/{wallet_id}/
├── meta.json     ← WalletMetadata (user_id, address, label, status, created_at)
└── key.pem       ← Encrypted secp256k1 private key (sealed by Gramine)
```

The PEM file on disk is Gramine-encrypted ciphertext. Without the SGX enclave, it is unreadable.

### 3. Signing

When `POST /v1/wallets/{id}/send` is called:

```
Inside SGX enclave:
  │
  ▼
Load /data/wallets/{wallet_id}/key.pem
  (Gramine decrypts transparently)
  │
  ▼
Construct EIP-1559 transaction (Alloy)
  │
  ▼
Sign transaction with k256 (ECDSA secp256k1)
  │
  ▼
Broadcast signed transaction to Avalanche RPC
  │
  ▼
Return tx_hash to caller
Private key zeroized from memory after use
```

### 4. Deletion

When `DELETE /v1/wallets/{id}` is called:

- Wallet status set to `deleted` in metadata
- Key file **remains on disk** (soft delete)
- No further signing is possible (ownership check blocks all operations on deleted wallets)

---

## Reserve Wallet Key

The reserve (service) wallet holds `MINTER_ROLE` on the rEUR contract. It is automatically bootstrapped by the enclave on first startup if no reserve wallet exists.

```
Enclave startup:
  │
  ▼
Check /data/system/fiat_service_wallet/meta.json
  │
  ├── Exists → load key, verify balance
  └── Missing → generate new secp256k1 key pair
               persist to /data/system/fiat_service_wallet/
               return address to admin for MINTER_ROLE grant
```

The reserve wallet address is visible via `GET /v1/admin/fiat/service-wallet`. The corresponding private key never leaves the enclave.

---

## Enclave Signing Key

The enclave signing key is distinct from wallet keys. It signs the SGX SIGSTRUCT:

- **Algorithm:** RSA 3072-bit with exponent 3 (Intel requirement)
- **Purpose:** Establishes MRSIGNER identity; required to build and run the enclave
- **Location:** `~/.config/gramine/enclave-key.pem` (default), or `SGX_SIGNING_KEY` env var
- **CI:** The `ENCLAVE_SIGNING_KEY` GitHub Actions secret holds a base64-encoded version

Compromise of the enclave signing key allows deploying an enclave with the same MRSIGNER. This is why MRENCLAVE (code hash) is the primary verification identity, not MRSIGNER.
{: .warning }

Generate a new signing key:

```bash
gramine-sgx-gen-private-key
# Written to ~/.config/gramine/enclave-key.pem
```

---

## Storage Encryption

Gramine's encrypted filesystem handles storage encryption transparently:

| Environment | Key derivation | Key lifetime |
|:------------|:---------------|:-------------|
| Development | Loaded from `data/.dev_storage_key` file | Persistent across restarts |
| Production | Derived from `_sgx_mrsigner` | Bound to enclave signer identity |

**Production note:** If the enclave signing key changes, data encrypted under the old key becomes permanently inaccessible. Plan signing key rotation carefully.

---

## What Is Never Exposed

| Data | API Endpoint | Accessible? |
|:-----|:-------------|:------------|
| Wallet private key (PEM) | None | Never |
| Reserve wallet private key | None | Never |
| Storage encryption key | None | Never |
| RA-TLS private key | None | Never (ephemeral, in-memory) |
| Enclave signing key | None | Never via API |

The only key-related output of the API is:
- `public_address` (Ethereum address derived from public key) — returned on wallet creation and read
- Reserve wallet `public_address` — returned by `GET /v1/admin/fiat/service-wallet`
