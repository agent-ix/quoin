---
id: US-003
title: "Install and use a community spec module"
type: US
relationships:
  - target: "ix://agent-ix/quoin/FR-011"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-018"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-019"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/StR-002"
    type: "traces_to"
---

# US-003: Install and use a community spec module

## Story

**As a** user adopting a community module
**I want** to install the module once under `~/.ix/filament/modules` (recorded in `~/.ix/filament/registry.json`), confirm that its types appear in the active catalog, and request those types in an authoring pack
**So that** the new vocabulary participates in authoring and validation just like the default modules.

## Context

A user extends the spec vocabulary with an additional artifact/object module
from a local path, GitHub repository, or npm package.

The expected command shape is:

```bash
quoin plugin install github:agent-ix/spec-objects-custom
quoin catalog list
quoin write . --types custom-object
```

The installed module should participate in the same authoring and validation
flow as the default modules. `quoin` maps the source argument to a typed source
and installs it. Package sources are covered by
[FR-187](../functional/FR-187-install-modules-from-npm-packages.md).

## Dependencies

- Plugin install records are stored in `~/.ix/filament/registry.json`; modules
  are materialized under `~/.ix/filament/modules`.
- Catalog loading merges `QUOIN_MODULE_PATHS` overrides with installed modules
  deterministically (env paths first).
- Install records and materialized modules use the shared registry and module
  directory described by FR-019.
