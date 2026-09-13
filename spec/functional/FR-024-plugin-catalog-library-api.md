---
id: FR-024
title: "Expose plugin and catalog operations as a stable library API"
relationships:
  - target: "ix://agent-ix/quoin/StR-003"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/US-003"
    type: "implements"
type: FR
---

# FR-024: Expose plugin and catalog operations as a stable library API

## Description

The package SHALL export its plugin install/list/remove and catalog-loading
operations — `installPlugin`, `listPlugins`, `removePlugin`, `parseSourceArg`,
`loadCatalog`, and `filamentModulesDir` — as a stable, semver-governed library API
from `@agent-ix/quoin`, so that other Node tools can manage and read the same
shared plugin store (`~/.ix/filament/registry.json` and `~/.ix/filament/modules/`)
identically to the `quoin` CLI, in service of
[StR-003](../stakeholder/StR-003-shared-catalog.md).

The durable cross-tool contract is the **shared store layout** plus the
`@agent-ix/ts-plugin-kit` source/registry model: a consumer may either import this
library (Node tools) or interoperate at the store level over the same registry and
module directories (as filament-ide does, to avoid pulling quoin's CLI dependency
tree into a desktop bundle). Either way an install/removal by one tool is observed
by the other.

This formalizes the existing exports as a consumed contract: the CLI behavior in
[FR-019](./FR-019-manage-plugin-registry.md) and the catalog assembly in
[FR-007](./FR-007-assemble-module-roots.md) become reusable from a library import,
not only the command line.

## Behavior

- The package entry point SHALL export `installPlugin`, `listPlugins`,
  `removePlugin`, `parseSourceArg`, `loadCatalog`, and `filamentModulesDir`.
- The exported operations SHALL read and write the same shared store paths as the
  CLI, so an install or removal performed through the library is observable by the
  CLI and vice versa.
- The package SHALL treat a breaking change to this exported surface as a
  semver-major change, so downstream consumers can pin against it.

## Acceptance Criteria

| ID          | Criteria                                                                                             | Verification           |
| ----------- | ---------------------------------------------------------------------------------------------------- | ---------------------- |
| FR-024-AC-1 | The package entry point exports the named plugin and catalog operations                              | Test (index.test.ts)   |
| FR-024-AC-2 | A plugin installed through `installPlugin` is listed by `listPlugins` and assembled by `loadCatalog` | Test (index.test.ts)   |
| FR-024-AC-3 | A library-performed install/removal targets the same shared store the CLI uses                       | Test (index.test.ts)   |

## Dependencies

- **Upstream**: [StR-003](../stakeholder/StR-003-shared-catalog.md) shared
  catalog; the CLI plugin operations [FR-019](./FR-019-manage-plugin-registry.md)
  and catalog assembly [FR-007](./FR-007-assemble-module-roots.md) whose internals
  this surface re-exports.
- **Downstream**: filament-ide interoperates over the same shared store via
  `@agent-ix/ts-plugin-kit` and these store conventions — rather than importing this
  library — to install, list, remove, and load plugins
  (`ix://agent-ix/filament-ide/FR-023`, `ix://agent-ix/filament-ide/FR-020`).
