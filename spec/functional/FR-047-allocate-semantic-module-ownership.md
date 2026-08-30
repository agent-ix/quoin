---
id: FR-047
title: "Allocate semantic-module subsystem ownership"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-013"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
---

# FR-047: Allocate semantic-module subsystem ownership

## Description

When a semantic-module capability is placed, the architecture record SHALL allocate it among
Quire, Quoin, `filament-core-data`, module repositories, and consuming applications according to
the capability's owned inputs and outputs.

## Rationale

The semantic-data program adds compiler and generated-package concerns beside the existing parser,
catalog, module, and consumer responsibilities. The placement must extend existing boundaries rather
than absorbing them into the newest component.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-047-AC-1 | Quire owns typed-Markdown parsing, validation, extraction, addressing, and byte-splicing, and does not own template rendering, schema-package publication, cross-language generation, catalog sourcing, or application policy. | Test (TC-1129) |
| FR-047-AC-2 | Quoin owns catalog discovery, locks, installation, update, authoring contracts, skills, workflows, and evidence-oriented orchestration, and does not own parser semantics, compiler implementation, runtime persistence, or consumer adapters. | Test (TC-1130) |
| FR-047-AC-3 | `filament-core-data` owns the shared semantic kernel, IR, compatibility rules, compiler, and emitters; module repositories own vocabulary, constraints, archetypes, mappings, examples, and semantic versions. | Test (TC-1131) |
| FR-047-AC-4 | Consumers own application adapters, API/IPC projections, persistence mappings, migrations, runtime state, and UI presentation without forking shared semantic identities. | Test (TC-1132) |
| FR-047-AC-5 | The record preserves ADR-0011's L0/L1/L2 validation levels, Validator/Advisor/Generator/Auditor roles, and rule that consumer CI executes L2 work. | Test (TC-1133) |

## Constraints

- Shared contracts may be consumed across owners; consumption does not transfer authority.
- A future owner change requires a reviewed successor ADR rather than an implicit implementation
  dependency.

## Dependencies

- **Upstream**: [US-013](../usecase/US-013-reason-about-semantic-module-boundaries.md),
  [StR-003](../stakeholder/StR-003-shared-catalog.md)
- **External basis**: `filament-core-data` ARCH-004 and Quire ADR-0011
