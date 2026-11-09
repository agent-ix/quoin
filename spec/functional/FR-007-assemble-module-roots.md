---
id: FR-007
title: "Assemble module roots in a defined order"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-002"
    type: "implements"
---

# FR-007: Assemble module roots in a defined order

## Description

The catalog loader SHALL assemble its candidate module roots in a defined order:
first the colon-separated entries of the `QUOIN_MODULE_PATHS` environment
variable (with empty entries dropped), then every module directory under
`~/.ix/filament/modules` — the shared module store also read by `quire`.

## Inputs

- The `QUOIN_MODULE_PATHS` environment variable and the contents of the shared
  module store under the resolved config root.

## Outputs

- An ordered list of candidate module roots.

## Behavior

- The loader SHALL split `QUOIN_MODULE_PATHS` on `:`, drop empty entries, and
  place those roots first.
- The loader SHALL then append every immediate directory under
  `<config-root>/filament/modules`.
- When the shared module store does not exist, the loader SHALL contribute only
  the environment-variable roots.

## Acceptance Criteria

| ID          | Criteria                                                                            | Verification           |
| ----------- | ----------------------------------------------------------------------------------- | ---------------------- |
| FR-007-AC-1 | `QUOIN_MODULE_PATHS` entries and installed module dirs are both included, env first | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-007-AC-2 | Empty `QUOIN_MODULE_PATHS` entries are dropped                                      | None (no native test is tagged to this criterion; PLAT-1045) |
| FR-007-AC-3 | With no installed modules, only env roots are contributed                           | None (no native test is tagged to this criterion; PLAT-1045) |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared catalog;
  [US-002](../usecase/US-002-discover-authoring-contract.md). The store location
  derives from [FR-004](./FR-004-resolve-config-root.md).
- **Downstream**: the ordering feeds first-wins deduplication in
  [FR-008](./FR-008-deduplicate-modules.md).
