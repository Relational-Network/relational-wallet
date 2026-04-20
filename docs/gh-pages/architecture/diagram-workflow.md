---
layout: default
title: Diagram Workflow
parent: Architecture
nav_order: 4
---

# Diagram Workflow
{: .fs-7 }

How to author, preview, and render PlantUML sequence and architecture diagrams.
{: .fs-5 .fw-300 }

---

## Directory Structure

```
docs/
├── sequence/            # PlantUML sources (.puml)
│   ├── push_transfer.puml
│   ├── pull_transfer.puml
│   ├── fiat_onramp.puml
│   ├── fiat_offramp.puml
│   ├── user_registration_kyc.puml
│   ├── cash_exchange.puml
│   └── STATUS.md        # Diagram status tracking
│
├── includes/
│   └── style.puml       # Shared PlantUML styles
│
├── architecture/
│   └── relational-wallet.pdf   # Exported architecture diagrams
│
├── seq-diagrams/        # Rendered PNG/SVG output
│
└── render.sh            # Rendering script
```

---

## Authoring

### Prerequisites

- [VS Code](https://code.visualstudio.com/) with the [PlantUML extension](https://marketplace.visualstudio.com/items?itemName=jebbs.plantuml)
- Java runtime (required by PlantUML renderer)
- Or use the [PlantUML online server](https://www.plantuml.com/plantuml/uml) for quick previews

### Workflow

1. Open the `docs/` folder in VS Code
2. Edit `.puml` files in `docs/sequence/`
3. Preview with <kbd>Alt</kbd> + <kbd>D</kbd> (VS Code PlantUML extension)
4. Include shared styles at the top of each diagram:

```plantuml
@startuml
!include ../includes/style.puml

' Your diagram content here

@enduml
```

### Style Guide

- Use the shared `style.puml` for consistent colors and fonts
- Name participants clearly (e.g., `participant "Wallet Web" as WEB`)
- Add notes for non-obvious steps
- Group related interactions with `== Section Name ==` dividers

---

## Rendering

Generate PNG/SVG exports from all `.puml` sources:

```bash
cd docs
./render.sh
```

Output is written to `docs/seq-diagrams/`.

### Available Diagrams

| Diagram | Source | Description |
|:--------|:-------|:------------|
| Push Transfer | `sequence/push_transfer.puml` | Standard send flow |
| Pull Transfer | `sequence/pull_transfer.puml` | Payment link / request flow |
| Fiat On-Ramp | `sequence/fiat_onramp.puml` | EUR deposit via TrueLayer |
| Fiat Off-Ramp | `sequence/fiat_offramp.puml` | EUR withdrawal via TrueLayer |
| User Registration | `sequence/user_registration_kyc.puml` | Sign-up and identity flow |
| Cash Exchange | `sequence/cash_exchange.puml` | In-person exchange flow |

---

## Adding New Diagrams

1. Create a new `.puml` file in `docs/sequence/`
2. Include the shared style: `!include ../includes/style.puml`
3. Preview with VS Code (<kbd>Alt</kbd> + <kbd>D</kbd>)
4. Run `./render.sh` to generate exports
5. Update `docs/sequence/STATUS.md` with the new diagram
6. Commit both the source and rendered output
