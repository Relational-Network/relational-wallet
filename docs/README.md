# Relational Wallet SDK Documentation

This folder hosts two things:

## 1. Diagram Sources

**PlantUML and PNG/SVG exports** that stay in `docs/`.

### Structure

- `sequence/`: Source PlantUML sequence diagram files (`.puml`)
- `architecture/`: Architecture diagram exports
- `includes/`: Shared styles and configuration
- `seq-diagrams/`: Output of rendered images

### How to Edit

1. Open this folder in VS Code.
2. Install the **PlantUML** extension.
3. Edit `.puml` files in `sequence/`.
4. Press `Alt + D` to preview.

### Rendering Diagrams

To render all `.puml` files in `sequence/` to PNG (and SVG):

```bash
./docs/render.sh
```

The generated output will appear in the `seq-diagrams/` directory.

## 2. GitHub Pages Content

Broader Wallet SDK documentation under [`docs/gh-pages/`](gh-pages/).
This includes installation, API, contracts, architecture, operations, and legal sections.

### Local Preview

Ensure you have Ruby ≥ 3.1 and Bundler 2.5.x installed (`gem install bundler -v 2.5.21 --user-install` if needed).

```bash
cd docs/gh-pages
bundle install
bundle exec jekyll serve --livereload --source . --destination ../_site
```

The site is now available at http://127.0.0.1:4000. Changes to files inside `docs/gh-pages/` hot-reload automatically. Stop the server with `Ctrl + C` when finished.

**Note:** Diagrams remain under `../sequence/` and `../architecture/`, so run `../render.sh` from the repo root whenever you modify `.puml` sources.

### GitHub Pages

1. Open **Settings → Pages** on GitHub.
2. **Source**: Deploy from a branch.
3. **Branch**: `main`, **Folder**: `/docs/gh-pages`.
4. **Save** — GitHub builds the Jekyll site using `_config.yml` and automatically redeploys after each push to `main`.

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

You may copy, modify, and redistribute this work under the terms of the AGPL-3.0. A full copy of the license can be found in the `LICENSE` file or at:

👉 https://www.gnu.org/licenses/agpl-3.0.html
