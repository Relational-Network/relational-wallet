---
layout: default
title: Docs Publishing
parent: Operations
nav_order: 1
---

# Docs Publishing

## Local Preview

```bash
cd docs/gh-pages
bundle install
bundle exec jekyll serve --source . --destination ../_site --livereload
```

Site runs at `http://127.0.0.1:4000`.

## Production (GitHub Pages)

Repository is configured to publish from:

- Branch: `main`
- Folder: `/docs/gh-pages`

Check in GitHub:

1. `Settings` -> `Pages`
2. Source: `Deploy from a branch`
3. Branch/folder match values above

## Pre-publish Checklist

1. `bundle exec jekyll build --source . --destination ../_site`
2. Verify no unintended placeholders:
   - `rg -n "Insert actual text" docs/gh-pages/legal -g '*.md'`
3. Verify current endpoint references:
   - `rg -n "/v1/fiat|/v1/admin/fiat|/wallets/bootstrap|/callback|/pay" docs/gh-pages -g '*.md'`

## Diagram Regeneration

If sequence diagrams changed:

```bash
./docs/render.sh
```
