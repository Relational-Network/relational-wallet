---
layout: default
title: Docs Publishing
parent: Operations
nav_order: 1
---

# Docs Publishing & CI/CD

## GitHub Pages (Recommended)

1. Open **Settings → Pages** on GitHub.
2. Source: `Deploy from a branch`.
3. Branch: `main`, Folder: `/docs/gh-pages`.
4. Save — GitHub builds the Jekyll site using `_config.yml` and automatically redeploys after each push to `main`.

> TODO