---
id: FR-008
title: "Deduplicate module roots and names first-wins"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-002"
    type: "implements"
---

# FR-008: Deduplicate module roots and names first-wins

## Description

The catalog loader SHALL include each resolved module root and each module name
only once, keeping the first occurrence, so that a module supplied earlier in the
assembly order — for example via `QUOIN_MODULE_PATHS` — overrides an installed
module that declares the same name.

## Behavior

- The loader SHALL skip a resolved module root it has already loaded.
- The loader SHALL skip a module whose declared name it has already loaded.
- Because `QUOIN_MODULE_PATHS` roots are assembled first
  ([FR-007](./FR-007-assemble-module-roots.md)), an environment-supplied module
  SHALL take precedence over an installed module of the same name.

## Acceptance Criteria

| ID          | Criteria                                                                       | Verification           |
| ----------- | ------------------------------------------------------------------------------ | ---------------------- |
| FR-008-AC-1 | A repeated module root is loaded only once                                     | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-008-AC-2 | A second module declaring an already-seen name is skipped, including its types | None (no native test is tagged to this criterion; PLAT-1045) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-002](../usecase/US-002-discover-authoring-contract.md); ordering from
  [FR-007](./FR-007-assemble-module-roots.md).
- **Downstream**: supports the deterministic-assembly guarantee in
  [NFR-002](../non-functional/NFR-002-deterministic-catalog.md).
