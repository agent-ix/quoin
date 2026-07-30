---
type: master-requirements
name: quoin
org: agent-ix
component_type: cli
implementation_language: typescript
title: "quoin Master Requirements Specification"
standards_alignment:
  - iso-iec-ieee-29148
relationships:
  - target: "ix://agent-ix/ix-cli-core"
    type: "depends_on"
  - target: "ix://agent-ix/ts-plugin-kit"
    type: "depends_on"
---

# Master Requirements Specification

## Purpose

`quoin` provides a standalone, installable CLI that gives a spec-authoring agent
everything it needs to start spec work: a catalog-driven authoring contract, a
user-extensible spec-module store, and launch points for governed
review/matrix/planning workflows. It runs on its own, so adopting spec-driven
development requires only this one tool on `PATH`.

This document is the top-level requirements artifact for the repository. It states
the scope and intent, indexes the requirement classes, and records how they trace
to one another. The authoritative requirements live as discrete files under
`spec/stakeholder/`, `spec/usecase/`, `spec/functional/`, `spec/non-functional/`,
and `spec/integration/`; this document indexes them.

## Scope

### In Scope

This specification governs:

- The `quoin` command surface: argument parsing; `version` and help; `catalog`
  list/show/validate; `write` authoring packs; `plugin`
  install/list/remove/ensure-defaults; the `review`/`matrix`/`to-plan` workflow
  launchers; and the `update` self-update command.
- Assembly of a Filament catalog from module roots (`QUOIN_MODULE_PATHS` and the
  installed module store) and the authoring contract it exposes — skeletons,
  schemas, and module roots.
- The committed default module set (`default-modules.yaml`) and its lazy,
  idempotent installation into `~/.ix/filament/modules`, plus user and community
  plugin records in `~/.ix/filament/registry.json`.
- Construction of the `quire validate` command an agent runs over authored spec
  files.

### Delegated Responsibilities

`quoin` collaborates with three external pieces and works with each as follows:

- **`ix-flow`** owns workflow lifecycle (resume, advance, gate, status) once a run
  is launched. `quoin` starts the run and hands control to `ix-flow`.
- **`quire`** owns frontmatter-driven validation of authored files. `quoin`
  constructs the scoped `quire validate` command and the agent runs it; both read
  the same shared module store, so the authoring contract matches the validation
  rules.
- **`@agent-ix/ts-plugin-kit`** owns install, registry, and reconcile mechanics.
  `quoin` maps CLI source arguments to typed sources and delegates the install.
- **`@agent-ix/ix-cli-core`** owns the runtime context, config-root resolution,
  and the npm-backed self-update. `quoin` builds on it and delegates the `update`
  command to its self-update primitive.

## System Overview

`quoin` is a single `main(argv)` dispatcher built on `@agent-ix/ix-cli-core`. It
parses a leading command (and, for `catalog` and `plugin`, a subcommand), resolves
a config root, configures the shared runtime context, and dispatches to the
catalog, authoring, plugin, workflow, or self-update surface.

It assembles a Filament catalog from module roots and returns the local
skeletons, schemas, and a scoped `quire validate` command that the calling agent
uses as its authoring contract. It installs user and community modules through
`@agent-ix/ts-plugin-kit`, and it launches review/matrix/planning workflows and
hands their lifecycle to `ix-flow`.

`quoin` declares its default module set in the committed `default-modules.yaml` as
pinned `git-subdir` sources and auto-installs that set on first catalog access
into the shared `~/.ix/filament/modules` store that `quire` also reads — so
authoring and validation see one identical catalog, and repeated work stays local.

## Requirements Architecture

Requirements are decomposed into the ISO/IEC/IEEE 29148 classes below. Each
requirement is a discrete file in its class directory and traces to its
neighbours: stakeholder needs (StR) drive user stories (US), which drive
functional requirements (FR); non-functional requirements (NFR) constrain the FRs;
and integration tests (IT) verify the boundaries `quoin` owns. This section is the
index; the files are authoritative.

### Stakeholder Requirements

- [StR-001](./stakeholder/StR-001-standalone-cli.md) — run spec work from a standalone CLI.
- [StR-002](./stakeholder/StR-002-extensible-vocabulary.md) — extend the vocabulary with community modules.
- [StR-003](./stakeholder/StR-003-shared-catalog.md) — authoring and validation share one catalog.
- [StR-004](./stakeholder/StR-004-governed-workflows.md) — review/matrix/planning run as governed workflows.
- [StR-005](./stakeholder/StR-005-offline-reproducible.md) — authoring stays offline-safe and reproducible.
- [StR-006](./stakeholder/StR-006-current-via-self-update.md) — keep quoin current with one command.

### User Stories

- [US-001](./usecase/US-001-author-root-artifact-with-objects.md) — author a root artifact with supporting objects.
- [US-002](./usecase/US-002-discover-authoring-contract.md) — discover an authoring contract before editing.
- [US-003](./usecase/US-003-install-community-module.md) — install and use a community spec module.
- [US-004](./usecase/US-004-validate-changed-spec-files.md) — validate changed spec files.
- [US-005](./usecase/US-005-start-gated-spec-workflow.md) — start a gated spec workflow.
- [US-006](./usecase/US-006-detect-conflicting-type-definitions.md) — detect conflicting type definitions across modules.
- [US-007](./usecase/US-007-review-into-specreview-docs.md) — review a spec into validated review docs.
- [US-008](./usecase/US-008-create-implementation-plan.md) — create an implementation plan from accepted requirements.
- [US-009](./usecase/US-009-install-in-any-coding-agent.md) — install quoin in the coding agent of my choice.
- [US-010](./usecase/US-010-author-specs-for-my-own-organization.md) — author specs for my own organization.

### Functional Requirements

**CLI framework**

- [FR-001](./functional/FR-001-parse-command-line.md) — parse the command line into command, flags, and positionals.
- [FR-002](./functional/FR-002-print-package-version.md) — report the installed package version.
- [FR-003](./functional/FR-003-print-usage-and-help.md) — print usage and command-scoped help.
- [FR-004](./functional/FR-004-resolve-config-root.md) — resolve the config root and configure the runtime context.
- [FR-005](./functional/FR-005-reject-unknown-commands.md) — reject unknown commands and subcommands.

**Catalog**

- [FR-006](./functional/FR-006-locate-module-roots.md) — locate a module root from a candidate path.
- [FR-007](./functional/FR-007-assemble-module-roots.md) — assemble module roots in a defined order.
- [FR-008](./functional/FR-008-deduplicate-modules.md) — deduplicate module roots and names first-wins.
- [FR-009](./functional/FR-009-read-module-manifest.md) — read each module manifest into catalog entries.
- [FR-010](./functional/FR-010-case-insensitive-type-lookup.md) — look up catalog types case-insensitively.
- [FR-011](./functional/FR-011-list-and-show-catalog.md) — list the catalog and show a single type.
- [FR-012](./functional/FR-012-detect-duplicate-types.md) — detect duplicate type definitions and validate the catalog.

**Authoring**

- [FR-013](./functional/FR-013-resolve-requested-types.md) — resolve the requested types for an authoring pack.
- [FR-014](./functional/FR-014-emit-authoring-contract.md) — emit an authoring contract per resolved type.
- [FR-015](./functional/FR-015-emit-quire-validate-command.md) — emit a scoped Quire validation command.

**Module store and plugins**

- [FR-016](./functional/FR-016-default-modules-schema.md) — default module set manifest schema (`object: data_schema`).
- [FR-017](./functional/FR-017-reconcile-default-modules.md) — reconcile the default module set into the shared store.
- [FR-018](./functional/FR-018-map-plugin-sources.md) — map plugin source arguments to typed sources.
- [FR-019](./functional/FR-019-manage-plugin-registry.md) — install, list, and remove plugins through the registry.
- [FR-024](./functional/FR-024-plugin-catalog-library-api.md) — expose plugin and catalog operations as a stable library API.

**Workflows**

- [FR-020](./functional/FR-020-resolve-workflow-skills.md) — expose workflow launchers and resolve their skills.
- [FR-021](./functional/FR-021-launch-ix-flow-runs.md) — launch workflow runs through ix-flow.

**Self-update**

- [FR-022](./functional/FR-022-self-update.md) — upgrade quoin to the latest published release.

**Configuration**

- [FR-023](./functional/FR-023-runtime-configuration.md) — quoin runtime configuration surface (`object: configuration`).
- [FR-025](./functional/FR-025-resolve-authoring-organization.md) — resolve the authoring organization.
- [FR-026](./functional/FR-026-dispatch-through-oclif-runner.md) — dispatch commands through the oclif runner.

### Non-Functional Requirements

- [NFR-001](./non-functional/NFR-001-idempotent-offline-reconcile.md) — default-module reconciliation is idempotent and offline-safe.
- [NFR-002](./non-functional/NFR-002-deterministic-catalog.md) — catalog assembly is deterministic.
- [NFR-003](./non-functional/NFR-003-actionable-errors.md) — failures surface as actionable errors.
- [NFR-004](./non-functional/NFR-004-standalone-dependencies.md) — quoin runs as a standalone package.
- [NFR-005](./non-functional/NFR-005-catalog-driven-workflows.md) — workflows reference catalog-defined types.
- [NFR-006](./non-functional/NFR-006-eval-metric-capture.md) — the agent eval set captures efficiency metrics.
- [NFR-007](./non-functional/NFR-007-external-tool-invocation.md) — external tools are invoked by name and surface their failures.
- [NFR-008](./non-functional/NFR-008-strict-manifest-parsing.md) — corrupt manifests abort assembly rather than drop silently.

### Integration Tests

- [IT-001](./integration/IT-001-default-module-reconcile.md) — default module set reconciles from pinned git tags, then serves offline.
- [IT-002](./integration/IT-002-github-plugin-install.md) — community plugin installs from a GitHub source into the catalog.

### Verification Layers

Three layers verify this specification, each named in the requirements above:

- **Unit tests** — deterministic command and module behaviour, mapped requirement
  to test in [matrix.md](./matrix.md) (`make test`, 100% coverage).
- **Agent evals** — the agent-pty matrix [TM-002](./evals.md) drives the real
  agent through `quoin` + `quire` end to end and records efficiency metrics
  (NFR-006).
- **Integration tests** — the two live-git boundaries `quoin` owns
  (`spec/integration/`).

## Module Store

`quoin` declares its default `spec-artifacts-*` and `spec-objects-*` modules in
the committed `default-modules.yaml` as pinned `git-subdir` sources, and installs
that set on first catalog access into `~/.ix/filament/modules/<name>/` — lazily,
idempotently, and (once pinned and installed) entirely offline. The install is
triggered by the `catalog` and `write` commands and on demand via
`quoin plugin ensure-defaults`, the explicit entry point external tools such as
`quire validate` use to bootstrap the set. User and community plugins are recorded
in `~/.ix/filament/registry.json` and materialized into the same store. Because
that store is the one `quire` reads, installs and validation see an identical
catalog.

## References

- ISO/IEC/IEEE 29148 — Requirements engineering.
- `@agent-ix/ix-cli-core` — runtime context, config root, and self-update.
- `@agent-ix/ts-plugin-kit` — marketplace install, reconcile, and registry
  primitives for the default set and for user and community plugins.
- `ix-flow` — workflow lifecycle (resume/advance/gate/status) after launch.
- `quire` — frontmatter-driven validation over authored spec files.
