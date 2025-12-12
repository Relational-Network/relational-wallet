---
layout: default
title: Diagram Workflow
parent: Architecture
nav_order: 2
---

# Diagram Workflow

Use this workflow when editing the PlantUML sources.

## Directory Structure

- `docs/sequence/` — PlantUML sequence diagrams (`.puml`).
- `docs/architecture/` — High-level architecture diagrams (PDF/SVG exports).
- `docs/includes/` — Shared styles.
- `docs/seq-diagrams/` — Generated PNG/SVG assets.

## Authoring Tips

1. Open the `docs/` folder in VS Code.
2. Install the **PlantUML** extension.
3. Edit `.puml` files directly (extension hotkeys such as <kbd>Alt</kbd> + <kbd>D</kbd> preview diagrams).

## Rendering

```
./docs/render.sh
```