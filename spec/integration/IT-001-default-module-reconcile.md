---
id: IT-001
title: "Default module set reconciles from pinned git tags, then serves offline"
type: IT
relationships:
  - target: "ix://agent-ix/quoin/FR-016"
    type: "verifies"
  - target: "ix://agent-ix/quoin/FR-017"
    type: "verifies"
  - target: "ix://agent-ix/quoin/NFR-001"
    type: "verifies"
---

# IT-001: Default module set reconciles from pinned git tags, then serves offline

## Objective

Verify the real integration boundary between `quoin`'s default-module reconcile
and the GitHub git remotes named in `default-modules.yaml`: on first catalog
access the pinned modules are checked out into the shared store, and on subsequent
access they are served from the store with no further git or network work. This
exercises the live git boundary that unit tests deliberately replace with
path-source fixtures.

## Target Integration

The component under test is `quoin`'s reconcile path (`ensureDefaultModules`,
backed by `@agent-ix/ts-plugin-kit`). The external dependency is the set of GitHub
repositories declared as `git-subdir` sources in `default-modules.yaml`. The
integration exercised is a git checkout of each module's pinned tag into
`<config-root>/filament/modules`.

## Preconditions

An isolated, empty config root (a fresh `IX_HOME`) so no module is pre-installed;
network access to GitHub for the first run; and the committed `default-modules.yaml`
present with its pinned sources.

## Inputs

The committed `default-modules.yaml` manifest — ten modules, each a `git-subdir`
source pinned to an immutable release reference or commit.

## Test Procedure

Each step performs one discrete action and has its own success criterion.

1. With the empty config root, trigger a first catalog access (`quoin catalog
list`).
   - IT-001-SC-01: each declared module is materialized under
     `<config-root>/filament/modules/<name>/` with a readable `manifest.yaml` at
     its pinned version.
2. Inspect the assembled catalog.
   - IT-001-SC-02: the catalog lists all ten default modules and resolves a
     known type (for example `FR`) to `spec-artifacts-iso`.
3. Remove network access and repeat the catalog access.
   - IT-001-SC-03: the command succeeds with no git or network activity and
     produces a catalog identical to step 2.

## Expected Results

The first access checks out every pinned module into the shared store and yields a
complete catalog; the second access, offline, reproduces that catalog with no
further git work. The test passes only when every per-step success criterion
holds.

## Metadata

- Priority: High
- Target Integration: GitHub `git-subdir` module sources
- Automation: Automated (live-git integration; currently spec-ahead-of-code — see
  Notes)

## Dependencies

**Upstream**: [FR-016](../functional/FR-016-default-modules-schema.md) and
[FR-017](../functional/FR-017-reconcile-default-modules.md), which this verifies;
the offline-safety guarantee [NFR-001](../non-functional/NFR-001-idempotent-offline-reconcile.md).
**Downstream**: none.

## Notes

The agent-pty eval harness exercises this boundary with modules **seeded once**
into an isolated `IX_HOME` (`evals/.seed-cache/`) rather than over live git, and
the unit suite uses path-source fixtures. A deterministic test that performs the
real first-run git checkout and the offline repeat does not yet exist in
`tests/`; this IT defines that coverage and is recorded as a backsync gap.
