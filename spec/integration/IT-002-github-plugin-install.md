---
id: IT-002
title: "Community plugin installs from a GitHub source into the catalog"
type: IT
relationships:
  - target: "ix://agent-ix/quoin/FR-018"
    type: "verifies"
  - target: "ix://agent-ix/quoin/FR-019"
    type: "verifies"
---

# IT-002: Community plugin installs from a GitHub source into the catalog

## Objective

Verify the real integration boundary between `quoin plugin install` and a GitHub
repository: a module installed from a `github://` source is checked out, recorded
in the registry, materialized into the shared store, and thereafter participates
in the catalog and in authoring contracts exactly like a default module. This
exercises the live GitHub boundary that the unit suite covers only with local
path sources.

## Target Integration

The component under test is `quoin`'s plugin install path (`installPlugin`, backed
by `@agent-ix/ts-plugin-kit`). The external dependency is a GitHub repository
hosting a spec module. The integration exercised is a git checkout of a
repository (or monorepo subdirectory) into `<config-root>/filament/modules`, plus
a registry write.

## Preconditions

An isolated, empty config root (a fresh `IX_HOME`); network access to GitHub; and
a reachable public module repository — for example
`agent-ix/spec-objects-security//spec_objects_security` at a published tag.

## Inputs

A single `github://` source argument naming the module repository, monorepo
subdirectory, and pinned ref, for example
`github:agent-ix/spec-objects-security//spec_objects_security@v0.2.0`.

## Test Procedure

Each step performs one discrete action and has its own success criterion.

1. Run `quoin plugin install <github-source>` against the empty config root.
   - IT-002-SC-01: the module is materialized under
     `<config-root>/filament/modules/<name>/manifest.yaml` and recorded in
     `<config-root>/filament/registry.json`.
2. List the catalog (`quoin catalog list`).
   - IT-002-SC-02: the installed module appears among the catalog modules.
3. Show a type the module declares (`quoin catalog show <type>`).
   - IT-002-SC-03: the type resolves to the installed module.
4. Build an authoring contract for that type (`quoin write . --types <type>`).
   - IT-002-SC-04: the contract returns the module's skeleton (and schema, when
     declared) for the type.

## Expected Results

The community module is installed from GitHub, registered, and fully usable: its
types appear in the catalog and resolve into authoring contracts. The test passes
only when every per-step success criterion holds.

## Metadata

- Priority: High
- Target Integration: GitHub repository / monorepo-subdir module source
- Automation: Automated (live-git integration; currently spec-ahead-of-code — see
  Notes)

## Dependencies

**Upstream**: [FR-018](../functional/FR-018-map-plugin-sources.md) and
[FR-019](../functional/FR-019-manage-plugin-registry.md), which this verifies.
**Downstream**: the installed module joins the catalog assembled by
[FR-007](../functional/FR-007-assemble-module-roots.md).

## Notes

This GitHub boundary is exercised at the agent-eval layer (`spec/evals.md`,
TC-EV-003/TC-EV-009/TC-EV-010/TC-EV-020), while the unit suite covers install/list/remove only
through local path sources. A deterministic test that performs the real GitHub
checkout does not yet exist in `tests/`; this IT defines that coverage and is
recorded as a backsync gap.
