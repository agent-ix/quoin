---
id: FR-006
title: "Locate a module root from a candidate path"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-002"
    type: "implements"
---

# FR-006: Locate a module root from a candidate path

## Description

The catalog loader SHALL treat a candidate path as a module root when the path
directly contains a `manifest.yaml`, or when one of its immediate child
directories contains a `manifest.yaml`, and SHALL skip candidates that do not
exist, are not directories, or resolve to no manifest.

## Inputs

- A candidate filesystem path.

## Outputs

- The resolved module root directory, or no result when the candidate yields no
  manifest.

## Behavior

- When the resolved candidate directly contains `manifest.yaml`, that directory
  SHALL be the module root.
- Otherwise, when an immediate child directory contains `manifest.yaml`, that
  child SHALL be the module root (the first such child, scanning in order).
- A candidate that does not exist, is a file rather than a directory, or contains
  no manifest at either level SHALL be skipped.

## Acceptance Criteria

| ID          | Criteria                                                             | Verification           |
| ----------- | -------------------------------------------------------------------- | ---------------------- |
| FR-006-AC-1 | A directory containing `manifest.yaml` resolves as a module root     | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-006-AC-2 | A manifest one level deep under a candidate is discovered            | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-006-AC-3 | A non-manifest child is skipped while a manifest sibling is found    | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-006-AC-4 | A missing path, an empty directory, and a file candidate are skipped | None (no native test is tagged to this criterion; PLAT-1045) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-002](../usecase/US-002-discover-authoring-contract.md).
- **Downstream**: [FR-007](./FR-007-assemble-module-roots.md) supplies the
  candidate paths this requirement resolves.
