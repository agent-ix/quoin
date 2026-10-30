---
id: FR-017
title: "Reconcile the default module set into the shared store"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-005"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-001"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-016"
    type: "references"
---

# FR-017: Reconcile the default module set into the shared store

## Description

The CLI SHALL lazily reconcile the default module set into
`~/.ix/filament/modules` on first `catalog` or `write` access — installing each
module idempotently and performing no git work once a module is installed — and SHALL expose a `plugin ensure-defaults` command as an explicit entry
point that performs the same install and reports the resulting registry for
external tools such as `quire validate`.

## Inputs

- The committed default-module manifest (see
  [FR-016](./FR-016-default-modules-schema.md)) and the resolved config root.

## Outputs

- The installed modules under `<config-root>/filament/modules` and, for
  `ensure-defaults`, a report of the resulting registry.

## Behavior

- On first `catalog`/`write` access, the CLI SHALL reconcile the default set into
  the shared store.
- Reconciliation SHALL be idempotent and SHALL skip git work for a module that is
  already installed, by name.
- `plugin ensure-defaults` SHALL run the reconcile explicitly and report the
  installed module names so external tools can trigger the bootstrap directly.

## Acceptance Criteria

| ID          | Criteria                                                                                      | Verification         |
| ----------- | --------------------------------------------------------------------------------------------- | -------------------- |
| FR-017-AC-1 | First catalog access installs the default modules into the shared store and loads their types | Test (index.test.ts) |
| FR-017-AC-2 | `plugin ensure-defaults` runs the installer and reports `{ensured:true, plugins}`             | Test (cli.test.ts)   |
| FR-017-AC-3 | `ensure-defaults` reports installed plugin names from a non-empty registry                    | Test (cli.test.ts)   |

## Dependencies

- **Upstream**: [StR-005](../stakeholder/StR-005-offline-reproducible.md);
  [US-001](../usecase/US-001-author-root-artifact-with-objects.md); the manifest
  schema [FR-016](./FR-016-default-modules-schema.md). Reconcile mechanics are
  provided by `@agent-ix/ts-plugin-kit`.
- **Downstream**: the idempotent, offline-safe behavior is constrained by
  [NFR-001](../non-functional/NFR-001-idempotent-offline-reconcile.md); live
  install is verified by
  [IT-001](../integration/IT-001-default-module-reconcile.md).
