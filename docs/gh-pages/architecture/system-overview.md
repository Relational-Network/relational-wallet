---
layout: default
title: System Overview
parent: Architecture
nav_order: 1
---

# System Overview

> Draft description of how the trusted enclave, wallet web, and Avalanche network fit together.

## Components

1. **Wallet Enclave** — Runs inside a Trusted Execution Environment, manages keys, signs Avalanche TXs, enforces policy.
2. **Wallet Web** — Browser client + lightweight backend using the enclave APIs.
3. **Avalanche Settlement** — On-chain ledger for stablecoin transfers.
