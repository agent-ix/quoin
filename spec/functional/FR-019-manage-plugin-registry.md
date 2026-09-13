---
id: FR-019
title: "Install, list, and remove plugins through the registry"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-003"
    type: "implements"
---

# FR-019: Install, list, and remove plugins through the registry

## Description

The CLI SHALL maintain user and community plugin records in
`~/.ix/filament/registry.json` and materialize modules by copy under
`~/.ix/filament/modules` through `ts-plugin-kit`, reading each module's name from
its `manifest.yaml` at the module root or a single nested directory; `plugin
install` SHALL record and report the installed plugin, `plugin list` SHALL print
the registry's plugins, and `plugin remove <name>` SHALL delete the module
directory and drop its registry entry.

## Inputs

- A `plugin install <source>`, `plugin list`, or `plugin remove <name>`
  invocation.

## Outputs

- The installed plugin record, the registry listing, or a removal confirmation.

## Behavior

- `plugin install` SHALL materialize the module by copy into the shared store,
  record it in the registry, and report the installed plugin.
- The CLI SHALL read a module's name from `manifest.yaml` at the root or the
  single nested directory, raising an error when no usable name is found.
- `plugin list` SHALL print the registry's plugins.
- `plugin remove <name>` SHALL delete the module directory and remove its registry
  entry, and SHALL raise an error when no name is given.

## Acceptance Criteria

| ID          | Criteria                                                                                                         | Verification                                       |
| ----------- | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| FR-019-AC-1 | `plugin install <path-source>` materializes the module, records it, and reports it                               | Test (install_reconcile.rs, core-modules.test.ts, cli.test.ts, index.test.ts) |
| FR-019-AC-2 | A module name is read from a root or single-nested `manifest.yaml`; a missing or empty name raises an error      | Test (ts_oracle.rs, install_reconcile.rs)          |
| FR-019-AC-3 | `plugin list` prints the registry's plugins                                                                      | Test (cli.test.ts)                                 |
| FR-019-AC-4 | `plugin remove <name>` deletes the module directory and drops its registry entry; a missing name raises an error | Test (install_reconcile.rs, cli.test.ts)           |

## Dependencies

- **Upstream**: [StR-002](../stakeholder/StR-002-extensible-vocabulary.md);
  [US-003](../usecase/US-003-install-community-module.md); typed sources from
  [FR-018](./FR-018-map-plugin-sources.md). Registry and install primitives are
  provided by `@agent-ix/ts-plugin-kit`.
- **Downstream**: installed modules join the catalog assembled by
  [FR-007](./FR-007-assemble-module-roots.md).
