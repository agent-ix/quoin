---
id: ARCH-SM-005
title: "External decision and compatibility ledger"
status: proposed
requirements:
  - FR-048
  - FR-050
  - NFR-013
---

# External decision and compatibility ledger

This ledger records the decisions issue #289 relies on. It does not promote a decision or alter
the status owned by another repository.

## External decisions

| Decision                       | Repository                    | Path                                                                                                        | Status                                                   | Disposition                                                                                                             |
| ------------------------------ | ----------------------------- | ----------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `ARCH-003`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/authority.md`                                                                    | normative                                                | preserved and specialized by FR-048                                                                                     |
| `ARCH-004`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/ownership.md`                                                                    | normative                                                | preserved and specialized by FR-047                                                                                     |
| `ARCH-005`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/metamodel.md`                                                                    | provisional; gate `filament-core-data#9`                 | plane distinctions preserved; exact future IR remains gated                                                             |
| `ARCH-006`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/generated-packages.md`                                                           | normative architecture contract; publication provisional | dynamic/static compatibility added; no package activated                                                                |
| `ADR-0005 / issue #4 decision` | `agent-ix/filament-core-data` | `docs/semantic-data-system/adr/0005-typespec-structural-source.md`; `spikes/typespec-feasibility/report.md` | ADR-0005 normative; ADR-0004 historical                  | TypeSpec is the structural source; JSON Schema and Protobuf are generated projections; spike evidence frozen            |
| `ADR-0002`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0002-three-layer-document-pipeline.md`                                                     | Draft                                                    | partially superseded: rendering responsibility is historical; parse/extract/address and byte-splicing remains preserved |
| `ADR-0003`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0003-unified-archetype-shape.md`                                                           | Proposed                                                 | preserved as a structural parsing model, not a universal semantic runtime base class                                    |
| `ADR-0004`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0004-markdown-default-validation.md`                                                       | Proposed                                                 | direct typed Markdown preserved; canonical Markdown within the document boundary clarified by current spec              |
| `ADR-0011`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0011-role-boundaries-validation-levels.md`                                                 | Accepted                                                 | governing for validation levels, roles, generated ownership, and consumer-CI execution                                  |
| `Quire current specification`  | `agent-ix/quire-rs`           | `spec/spec.md`; `docs/USAGE.md`; `README.md`                                                                | normative current behavior                               | canonical Markdown boundary and render removal govern over older draft language                                         |

## TypeSpec and structural schema status

TypeSpec is the structural source. The owner resolved filament-core-data ADR-0004 on
2026-09-03 (filament-core-data#4) and recorded the decision in ADR-0005, which is normative.
Modular TypeSpec packages plus versioned package, export, target, mapping, and profile metadata
are the accepted schema/package source; JSON Schema 2020-12 and Protobuf are official-emitter
projections. Compiler, IR, and emitters stay in `filament-core-data` (its ADR-0002).

The spike's custom Rust and TypeScript emitters and governed Python generator integration are
evidence of feasibility and maintenance cost, not production packages. Current Avro contracts and
consumers remain unchanged. Protobuf remains a fit-for-purpose wire projection and Arrow remains
an analytical projection.

## Quire reconciliation

### ADR-0002 — partially superseded

The three-stage parse/extract/address and byte-splice concepts remain useful. The rendering
responsibility is historical for Quire because the current specification explicitly removed
rendering. Quire does not regain templates or output generation through this architecture.

### ADR-0003 — preserved without promotion

The unified archetype shape remains a coherent structural parsing model for artifact/object
handling. It is not a universal semantic runtime base class and does not collapse structural kind,
semantic role, definition, occurrence, and projection.

### ADR-0004 — preserved and clarified

Direct typed Markdown remains the default authoring and validation model. The current spec's
canonical Markdown within the document boundary governs: derived JSON or database views do not
replace the source document for authored knowledge.

### ADR-0011 — accepted and governing

ADR-0011 is Accepted and remains governing for L0/L1/L2 validation, Validator/Advisor/Generator/
Auditor roles, optional heavy analysis, generated-artifact ownership, and consumer-CI execution.

## Change rule

External decision status changes occur in the owning repository first and are then reflected here.
