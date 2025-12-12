---
layout: default
title: Home
nav_order: 1
description: Overview of the Relational Wallet documentation space.
---

# Relational Wallet Documentation

Welcome to the documentation hub for the Relational Wallet. This site centralizes everything needed to build, and deploy the wallet.

## What's Inside

- **Installation** — Environment setup guides for the trusted execution environment [`(wallet-enclave)`](/installation/wallet-enclave) and the user-facing client [`(wallet-web)`](/installation/wallet-web).
- **API Documentation** — Component-level REST/WebSocket references and SDK notes organized per surface.
- **Architecture** — System overview, sequence diagrams, and tips for updating PlantUML diagrams.
- **Operations** — CI/CD, GitHub Pages publishing guidance, and operational runbooks.
- **Legal** — Privacy Policy and Terms of Service _placeholders_.

## Contributing to Docs

- Keep pages in Markdown, with front matter defining `title`, `nav_order`, and parent metadata so they appear correctly in the sidebar.
- Diagram source files live under `../architecture/` and `../sequence/`. Update them with PlantUML and regenerate assets via `../render.sh`.
