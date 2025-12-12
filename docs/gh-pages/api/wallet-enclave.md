---
layout: default
title: Wallet Enclave
parent: API Documentation
nav_order: 1
---

# Web Enclave API

> Placeholder for enclave RPC documentation.

## Authentication & Transport

- All calls travel over mutually-authenticated TLS within the secure enclave perimeter.
- Include remote attestation proof in the TLS handshake or via a dedicated header (spec TBD).

## Endpoints

| Endpoint | Method | Description | Notes |
| --- | --- | --- | --- |
| `/v1/wallet/create` | `POST` | Create a wallet + key material | TODO  |
| `/v1/wallet/{id}/sign` | `POST` | Request a signature | Enforces policy + rate limits |
| `/v1/attestation` | `GET` | Fetch attestation evidence | TODO |

## Events / Webhooks

