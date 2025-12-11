# Relational Wallet SDK Documentation

This repository contains the [architecture](architecture/) and [sequence](sequence/) diagrams for the Relational Wallet SDK

## Structure
* `sequence/`: Source PlantUML sequence diagram files (.puml)
* `architecture/`: Architecture diagrams
* `includes/`: Shared styles and configuration
* `seq-diagrams/`: Output of rendered images 

## How to edit
1. Open this folder in VS Code.
2. Install the **PlantUML** extension.
3. Edit files in `diagrams/`.
4. Press `Alt + D` to preview.

## Rendering Diagrams
To render all `.puml` files in `sequence/` to PNG (and SVG):

```./render.sh```

The generated output will appear in the `seq-diagrams/` directory.

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0).

You may copy, modify, and redistribute this work under the terms of the AGPL-3.0.
A full copy of the license can be found in the LICENSE file or at:

👉 https://www.gnu.org/licenses/agpl-3.0.html