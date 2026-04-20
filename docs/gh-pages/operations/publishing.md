---
layout: default
title: Docs Publishing
parent: Operations
nav_order: 2
---

# Docs Publishing
{: .fs-7 }

How to preview, build, and publish the documentation site.
{: .fs-5 .fw-300 }

---

## Technology

The documentation site is built with [Jekyll](https://jekyllrb.com/) using the [just-the-docs](https://just-the-docs.github.io/just-the-docs/) theme. It is deployed automatically to GitHub Pages on push to `main`.

---

## Local Preview

### Prerequisites

- Ruby 3.3+
- Bundler (`gem install bundler`)

### Setup (First Time)

```bash
cd docs/gh-pages
bundle install
```

### Start Dev Server

```bash
cd docs/gh-pages
bundle exec jekyll serve \
  --source . \
  --destination ../_site \
  --livereload

# Site runs at http://127.0.0.1:4000/relational-wallet/
```

`--livereload` automatically refreshes the browser when you save markdown files.

---

## Building for Production

```bash
cd docs/gh-pages
bundle exec jekyll build \
  --source . \
  --destination ../_site
```

Output is written to `docs/_site/`. The CI workflow builds this automatically.

---

## GitHub Pages Deployment

The site is deployed automatically by the `docs-site.yml` workflow on every push to `main` that touches `docs/gh-pages/**`.

### Verify GitHub Pages Settings

1. Go to the repository on GitHub
2. **Settings** → **Pages**
3. Confirm:
   - **Source:** Deploy from a branch
   - **Branch:** `main`
   - **Folder:** `/docs/gh-pages`

### Manual Trigger

If the deployment is stale, re-trigger from **Actions** → **docs-site** → **Re-run workflow**.

---

## Pre-Publish Checklist

Before merging documentation changes:

```bash
cd docs/gh-pages

# 1. Build without errors
bundle exec jekyll build --source . --destination ../_site
echo "Exit code: $?"

# 2. Check for placeholder text in legal section
grep -rn "TODO\|Insert actual\|placeholder" legal/ --include="*.md"

# 3. Verify no broken internal links (check key paths)
grep -rn "](/relational-wallet/" . --include="*.md" | head -30

# 4. Check all API endpoint references are current
grep -rn "/v1/" . --include="*.md" | grep -v "_site" | head -20

# 5. Regenerate diagrams if changed
cd ../../
./docs/render.sh
```

---

## Authoring Guidelines

| Rule | Rationale |
|:-----|:----------|
| Front matter `nav_order` must be unique per parent | just-the-docs uses this for sidebar ordering |
| All new sections need a parent `index.md` with `has_children: true` | Required for section navigation |
| Use `{: .note }`, `{: .warning }`, `{: .tip }` for callouts | Rendered as styled callout boxes |
| Code blocks with language hint (` ```bash `, ` ```json `) | Enables syntax highlighting |
| Use absolute permalink paths in links (`/relational-wallet/section/page`) | Works regardless of nesting level |

### nav_order Assignments

| Section | `nav_order` |
|:--------|:------------|
| Home | 1 |
| Installation | 2 |
| API Reference | 3 |
| Architecture | 4 |
| Smart Contracts | 5 |
| Security | 6 |
| User Guides | 7 |
| Operations | 8 |
| Legal | 9 |

---

## Adding a New Section

1. Create `docs/gh-pages/<section>/index.md` with:
   ```yaml
   ---
   layout: default
   title: Section Name
   nav_order: <next_number>
   has_children: true
   permalink: /<section>/
   ---
   ```
2. Create child pages with `parent: Section Name`
3. Update `nav_order` in `_config.yml` if needed
4. Run `bundle exec jekyll serve` and verify the sidebar
