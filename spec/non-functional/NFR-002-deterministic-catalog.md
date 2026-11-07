---
id: NFR-002
title: "Catalog assembly is deterministic"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-007"
    type: "constrains"
  - target: "ix://agent-ix/quoin/FR-008"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
---

# NFR-002: Catalog assembly is deterministic

## Statement

Catalog assembly SHALL be deterministic for a given set of module roots: a stable
root ordering, first-wins deduplication, and sorted duplicate-module lists, so
that the same inputs always yield the same catalog.

## Scope

- Applies to: catalog assembly from module roots and duplicate reporting.
- Operational context: every catalog read.

## Rationale

An author and a validator must see the same catalog from the same inputs.
Non-deterministic ordering or dedup would make authoring contracts and duplicate
reports vary run to run, undermining trust in the catalog.

## Measurement and Evaluation

| Metric                                                | Target | Threshold | Method |
| ----------------------------------------------------- | ------ | --------- | ------ |
| Catalog ordering variance across runs for fixed roots | 0      | 0         | Test   |
| Duplicate-module lists not in sorted order            | 0      | 0         | Test   |

## Verification

`quoin-cli` `tests/tc_catalog_command.rs` ::
`tc_521_catalog_methods_preserves_first_wins_data` asserts the first-wins projection. No
native test is cited for repeated roots or sorted duplicate-module lists (PLAT-1275).

## Dependencies

- **Upstream**: [FR-007](../functional/FR-007-assemble-module-roots.md) and
  [FR-008](../functional/FR-008-deduplicate-modules.md), whose behaviour this
  constrains.
- **Downstream**: none.
