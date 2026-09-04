---
type: master-requirements
name: quoin
org: agent-ix
component_type: cli
implementation_language: typescript
title: "quoin Master Requirements Specification"
quality_attributes_not_applicable:
  - availability
  - safety
  - compliance
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
  review/matrix/planning workflows; and read-only assurance and trace-graph
  analyses. It runs on its own, so adopting spec-driven
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
- The durable semantic-module architecture record that allocates authority and
  ownership across Quire, Quoin, `filament-core-data`, module repositories, and
  consumers without activating compiler, publication, or migration work.
- The read-only, denominator-closed semantic type-fit audit of the pinned default-module
  corpus and its canonical machine-readable findings and human report projection.
- Read-only fan-out, change-impact, and reaffirmation-churn views that consume a
  validated, source-grounded Quire assurance export and join it to Quoin's retained
  evidence and existing auditor verdicts without collecting evidence or defining policy.

* **Semantic module contract (issue #293):** the optional `semantic` manifest block, the typed-Properties/`sysml`-fence and Invariants/Operations mapping fixtures Quire implements, `data_schema` by path and digest, legacy forms at `warning`, and derived package manifests with registry pins — without compiling, publishing, or migrating anything.

### Delegated Responsibilities

`quoin` collaborates with three external pieces and works with each as follows:

- **`ix-flow`** owns workflow lifecycle (resume, advance, gate, status) once a run
  is launched. `quoin` starts the run and hands control to `ix-flow`.
- **`quire`** owns frontmatter-driven validation of authored files. `quoin`
  constructs the scoped `quire validate` command and the agent runs it; both read
  the same shared module store, so the authoring contract matches the validation
  rules. Quire also owns the authoritative artifact/relationship projection and
  publishes it as a versioned assurance export; Quoin validates that offline
  artifact before deriving graph-analysis views and does not re-parse frontmatter.
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
- [StR-008](./stakeholder/StR-008-conforming-module-repositories-by-construction.md) — new semantic-module repositories conform by construction.

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
- [US-011](./usecase/US-011-generate-property-tests-from-criteria.md) — generate property tests from acceptance criteria.
- [US-012](./usecase/US-012-generate-fuzz-harnesses.md) — generate fuzz harnesses for specified input surfaces.
- [US-013](./usecase/US-013-reason-about-semantic-module-boundaries.md) — reason about semantic modules without confusing definitions and projections.
- [US-014](./usecase/US-014-audit-default-module-semantic-fit.md) — audit the semantic fit of the complete default-module corpus.
- [US-015](./usecase/US-015-assess-intervention-experiments.md) — assess intervention experiments without overstating causality.
- [US-020](./usecase/US-020-declare-a-semantic-module-contract-once.md) — declare a module's semantic contract once against the shared grammar.
- [US-021](./usecase/US-021-generate-a-conforming-semantic-module-repository.md) — generate a semantic-module repository that already conforms to the contract.

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
- [FR-027](./functional/FR-027-store-the-authoring-organization.md) — store the authoring organization.

**Assurance evidence and reporting**

- [FR-028](./functional/FR-028-generate-property-tests-from-criteria.md) — generate property tests from classified criteria.
- [FR-029](./functional/FR-029-consume-the-quire-json-contract.md) — consume the published Quire JSON contract.
- [FR-030](./functional/FR-030-evidence-store.md) — retain the evidence store as the artifact of record.
- [FR-031](./functional/FR-031-catalog-driven-advisor.md) — advise verification methods from the catalog.
- [FR-032](./functional/FR-032-evidence-auditor.md) — audit freshness, suspicion, and vacuous evidence.
- [FR-033](./functional/FR-033-evidence-format-adapters.md) — transcribe supported evidence formats.
- [FR-034](./functional/FR-034-finding-shaped-evidence.md) — retain finding-shaped evidence.
- [FR-035](./functional/FR-035-combinatorial-coverage.md) — compute declared combinatorial coverage.
- [FR-036](./functional/FR-036-architecture-conformance.md) — verify architecture conformance.
- [FR-037](./functional/FR-037-declared-vocabulary-completeness.md) — verify declared-vocabulary completeness.
- [FR-038](./functional/FR-038-generate-fuzz-harnesses.md) — generate fuzz harnesses from Fuzz obligations.
- [FR-039](./functional/FR-039-mutation-score-threshold.md) — retain mutation score thresholds.
- [FR-040](./functional/FR-040-assurance-case-view.md) — render assurance cases as read-only store views.
- [FR-041](./functional/FR-041-sbom-inventory-evidence.md) — transcribe SBOM inventories.
- [FR-042](./functional/FR-042-agent-eval-evidence.md) — transcribe agent-eval reports.
- [FR-043](./functional/FR-043-quality-benchmark.md) — run the governed quality benchmark.
- [FR-044](./functional/FR-044-plan-governed-measurements.md) — store and report plan-governed measurements.
- [FR-045](./functional/FR-045-portfolio-measurement-report.md) — report measurement portfolios across repositories.

**Semantic-module architecture**

- [FR-046](./functional/FR-046-record-semantic-data-planes.md) — record the four semantic data planes.
- [FR-047](./functional/FR-047-allocate-semantic-module-ownership.md) — allocate subsystem ownership.
- [FR-048](./functional/FR-048-declare-authority-by-concern.md) — declare authority by concern.
- [FR-049](./functional/FR-049-preserve-dynamic-and-generated-modules.md) — preserve dynamic modules and finite generated packages.
- [FR-050](./functional/FR-050-reconcile-quire-decisions.md) — reconcile the architecture with Quire decisions.

**Semantic-module type-fit audit**

- [FR-051](./functional/FR-051-snapshot-semantic-audit-scope.md) — snapshot audit scope and provenance.
- [FR-052](./functional/FR-052-inventory-default-module-corpus.md) — inventory the complete default-module corpus.
- [FR-053](./functional/FR-053-score-semantic-type-fit.md) — score and disposition every declared type.
- [FR-054](./functional/FR-054-publish-semantic-audit-artifacts.md) — publish canonical audit artifacts.
- [FR-055](./functional/FR-055-reconcile-semantic-audit-findings.md) — reconcile findings with current contracts.

**Intervention-experiment evidence**

- [FR-056](./functional/FR-056-intervention-experiment-record.md) — define intervention-experiment evidence records.
- [FR-057](./functional/FR-057-intervention-experiment-intake-report.md) — ingest and report intervention-experiment evidence.
- [FR-058](./functional/FR-058-agent-eval-intervention-producer.md) — produce intervention evidence from retained real agent-evaluation runs.

**Operational evidence**

- [FR-059](./functional/FR-059-operational-evidence-records.md) — define standing-capability and exercise records.
- [FR-060](./functional/FR-060-operational-evidence-intake-report.md) — ingest operational evidence and evaluate clocked discharge.
- [FR-061](./functional/FR-061-github-actions-release-operational-producer.md) — produce linked operational records from a retained real GitHub Actions release run.


**Semantic module contract (issue #293)**

- [FR-070](./functional/FR-070-semantic-module-manifest-extension.md) — optional, versioned `semantic` manifest block.
- [FR-071](./functional/FR-071-typed-properties-mapping.md) — typed Properties table and SysML fence map to one `FieldDecl[]`.
- [FR-072](./functional/FR-072-invariants-and-operations-mapping.md) — Invariants and Operations map to `ClauseRef[]` and `OperationDecl[]`.
- [FR-073](./functional/FR-073-data-schema-by-path-and-digest.md) — `data_schema` by emitted-schema path and digest.
- [FR-074](./functional/FR-074-legacy-authoring-forms.md) — legacy authoring forms at `warning` with a declared migration.
- [FR-075](./functional/FR-075-semantic-package-exports-and-locks.md) — package exports, imports, locks, and generated coordinates.

**Semantic module cookiecutter (issue #307)**

- [FR-076](./functional/FR-076-semantic-module-template-variants.md) — artifact, object, and mixed variants from one maintained template core.
- [FR-077](./functional/FR-077-generated-schema-emission.md) — generated TypeSpec source and deterministic schema emission.
- [FR-078](./functional/FR-078-generated-manifest-semantic-block.md) — generated manifest `semantic` block and reference-form `data_schema`.
- [FR-079](./functional/FR-079-generated-skeletons-and-fixtures.md) — generated skeletons, mappings, and compatibility fixtures.
- [FR-080](./functional/FR-080-generated-verification-suite.md) — generated suite treats the engine as a hard dependency.
- [FR-081](./functional/FR-081-generated-public-repository-baseline.md) — generated public repository and packaging baseline.
- [FR-082](./functional/FR-082-generated-governance-tree.md) — generated governance tree validates as rendered.
- [FR-083](./functional/FR-083-template-render-self-tests.md) — template render self-tests and module conformance gate.

### Non-Functional Requirements

- [NFR-001](./non-functional/NFR-001-idempotent-offline-reconcile.md) — default-module reconciliation is idempotent and offline-safe.
- [NFR-002](./non-functional/NFR-002-deterministic-catalog.md) — catalog assembly is deterministic.
- [NFR-003](./non-functional/NFR-003-actionable-errors.md) — failures surface as actionable errors.
- [NFR-004](./non-functional/NFR-004-standalone-dependencies.md) — quoin runs as a standalone package.
- [NFR-005](./non-functional/NFR-005-catalog-driven-workflows.md) — workflows reference catalog-defined types.
- [NFR-006](./non-functional/NFR-006-eval-metric-capture.md) — the agent eval set captures efficiency metrics.
- [NFR-007](./non-functional/NFR-007-external-tool-invocation.md) — external tools are invoked by name and surface their failures.
- [NFR-008](./non-functional/NFR-008-strict-manifest-parsing.md) — corrupt manifests abort assembly rather than drop silently.
- [NFR-013](./non-functional/NFR-013-traceable-semantic-architecture.md) — decisions remain traceable and standalone-readable.
- [NFR-014](./non-functional/NFR-014-non-disruptive-architecture-record.md) — architecture recording remains non-disruptive.
- [NFR-015](./non-functional/NFR-015-complete-reproducible-semantic-audit.md) — audit completeness and reproducibility.
- [NFR-016](./non-functional/NFR-016-read-only-semantic-audit.md) — read-only, non-disruptive auditing.
- [NFR-017](./non-functional/NFR-017-non-disruptive-manifest-evolution.md) — semantic manifest evolution invalidates no current manifest or artifact.
- [NFR-018](./non-functional/NFR-018-rendered-output-hygiene.md) — rendered module repositories carry no generation residue.
- [NFR-019](./non-functional/NFR-019-deterministic-rendering.md) — rendering and schema regeneration are byte-deterministic.

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

`quoin` declares its default `engineering-assurance`, `spec-artifacts-*`, and `spec-objects-*` modules in
the committed `default-modules.yaml` as pinned `git-subdir` sources, and installs
that set on first catalog access into `~/.ix/filament/modules/<name>/` — lazily,
idempotently, and (once pinned and installed) entirely offline. The install is
triggered by the `catalog` and `write` commands and on demand via
`quoin plugin ensure-defaults`, the explicit entry point external tools such as
`quire validate` use to bootstrap the set. User and community plugins are recorded
in `~/.ix/filament/registry.json` and materialized into the same store. Because
that store is the one `quire` reads, installs and validation see an identical
catalog.

## Quality Characteristics Not Applicable

The ISO 25010 characteristics `spec-artifacts-iso` declares are swept by
`quoin completeness` (FR-037). Three do not apply to this component, and a
characteristic excused without a written reason is a `high` finding — so the
reasons are here rather than in a bare frontmatter list.

| Characteristic | Justification |
| -------------- | ------------- |
| availability | quoin is a command-line tool, not a service. It has no uptime, no session and no request to fail; a run either completes or exits non-zero, which reliability already governs. |
| safety | quoin actuates nothing. Its outputs are files and an exit code, and no physical process, medical device or vehicle function reads them, so no hazard analysis has a subject. |
| compliance | no regulatory regime governs a developer specification tool. Licensing obligations are real but are a distribution property of the package, not a quality characteristic of its behaviour. |

## References

- ISO/IEC/IEEE 29148 — Requirements engineering.
- `@agent-ix/ix-cli-core` — runtime context, config root, and self-update.
- `@agent-ix/ts-plugin-kit` — marketplace install, reconcile, and registry
  primitives for the default set and for user and community plugins.
- `ix-flow` — workflow lifecycle (resume/advance/gate/status) after launch.
- `quire` — frontmatter-driven validation over authored spec files.
