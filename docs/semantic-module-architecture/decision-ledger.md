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
the status owned by another repository. All source files were reviewed on 2026-08-29 at the
identity shown below.

## External decisions

| Decision                       | Repository                    | Path                                                                                                         | Status                                                   | Reviewed revision or date                               | Disposition                                                                                                             |
| ------------------------------ | ----------------------------- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `ARCH-003`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/authority.md`                                                                     | normative                                                | `d8984f2d61a9a912e20caa65115b353c5a14e9cc` (2026-08-29) | preserved and specialized by FR-048                                                                                     |
| `ARCH-004`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/ownership.md`                                                                     | normative                                                | `d8984f2d61a9a912e20caa65115b353c5a14e9cc` (2026-08-29) | preserved and specialized by FR-047                                                                                     |
| `ARCH-005`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/metamodel.md`                                                                     | provisional; gate `filament-core-data#9`                 | `d8984f2d61a9a912e20caa65115b353c5a14e9cc` (2026-08-29) | plane distinctions preserved; exact future IR remains gated                                                             |
| `ARCH-006`                     | `agent-ix/filament-core-data` | `docs/semantic-data-system/generated-packages.md`                                                            | normative architecture contract; publication provisional | `d8984f2d61a9a912e20caa65115b353c5a14e9cc` (2026-08-29) | dynamic/static compatibility added; no package activated                                                                |
| `ADR-0004 / issue #4 evidence` | `agent-ix/filament-core-data` | `docs/semantic-data-system/adr/0004-conditional-typespec-source.md`; `spikes/typespec-feasibility/report.md` | ADR provisional; evidence accepted                       | `90978f9b916999488452fd4fdfacfbe7216dc0c2` (2026-08-29) | modular JSON Schema fallback recommendation accepted for next design stage; TypeSpec unpromoted                         |
| `ADR-0002`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0002-three-layer-document-pipeline.md`                                                      | Draft                                                    | `0f4cc5fb94a344ef44a0ad66864bcd445c5db521` (2026-08-29) | partially superseded: rendering responsibility is historical; parse/extract/address and byte-splicing remains preserved |
| `ADR-0003`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0003-unified-archetype-shape.md`                                                            | Proposed                                                 | `0f4cc5fb94a344ef44a0ad66864bcd445c5db521` (2026-08-29) | preserved as a structural parsing model, not a universal semantic runtime base class                                    |
| `ADR-0004`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0004-markdown-default-validation.md`                                                        | Proposed                                                 | `0f4cc5fb94a344ef44a0ad66864bcd445c5db521` (2026-08-29) | direct typed Markdown preserved; canonical Markdown within the document boundary clarified by current spec              |
| `ADR-0011`                     | `agent-ix/quire-rs`           | `spec/assets/adr/0011-role-boundaries-validation-levels.md`                                                  | Accepted                                                 | `0f4cc5fb94a344ef44a0ad66864bcd445c5db521` (2026-08-29) | governing for validation levels, roles, generated ownership, and consumer-CI execution                                  |
| `Quire current specification`  | `agent-ix/quire-rs`           | `spec/spec.md`; `docs/USAGE.md`; `README.md`                                                                 | normative current behavior                               | `0f4cc5fb94a344ef44a0ad66864bcd445c5db521` (2026-08-29) | canonical Markdown boundary and render removal govern over older draft language                                         |

## TypeSpec and structural schema status

The completed feasibility spike recommends modular JSON Schema 2020-12 plus versioned package,
export, target, mapping, and profile metadata for the next design stage. TypeSpec remains
unpromoted. ADR-0004 remains provisional and still requires a separate human resolution before
TypeSpec can become an authoritative source or production dependency.

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

If any reviewed revision changes a relied-on statement, the architecture is suspect until a
maintainer compares the new decision and updates this ledger. A mutable branch tip or issue comment
is not a replacement for this identity. External decision status changes occur in the owning
repository first and are then reflected here.
