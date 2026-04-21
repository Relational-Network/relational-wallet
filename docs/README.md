# docs

Diagram sources, the GitHub Pages site, and the whitepaper.

| Path | Contents |
|------|----------|
| [`sequence/`](sequence/) | PlantUML sequence diagram sources (`.puml`) |
| [`architecture/`](architecture/) | Architecture diagram exports |
| [`includes/`](includes/) | Shared PlantUML styles |
| [`whitepaper/`](whitepaper/) | Project whitepaper sources |
| [`gh-pages/`](gh-pages/) | Jekyll site published to GitHub Pages |
| [`render.sh`](render.sh) | Renders all `.puml` files to PNG/SVG under `seq-diagrams/` |

## Diagrams

Edit `.puml` files in [`sequence/`](sequence/) (VS Code + the PlantUML extension gives `Alt+D` preview). Re-render after changes:

```bash
./docs/render.sh
```

## GitHub Pages site

Published from [`gh-pages/`](gh-pages/) by the workflow in [`.github/workflows/docs-site.yml`](../.github/workflows/docs-site.yml). Repo setting: **Settings → Pages → Source = GitHub Actions**.

Local preview (Ruby ≥ 3.1, Bundler 2.5.x):

```bash
cd docs/gh-pages
bundle install
bundle exec jekyll serve --livereload --source . --destination ../_site
```

Site at <http://127.0.0.1:4000>; livereloads on edits to `gh-pages/`. Diagrams under `../sequence/` and `../architecture/` need a manual `../render.sh` after `.puml` edits.

---

SPDX-License-Identifier: AGPL-3.0-or-later · Copyright (C) 2026 Relational Network
