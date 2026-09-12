---
id: FR-099
title: "Provide catalog, module and validation capability in Rust"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-098"
    type: "requires"
---

# FR-099: Provide catalog, module and validation capability in Rust

## Description

Quoin SHALL implement its Quire adapter, validators, semantic and completeness
analysis, configuration, plugin, module and catalog behaviour in Rust crates
reached through the `quoin-core` boundary, preserving one catalog and one module
store.

## Inputs

- Module manifests, schemas, skeletons and mappings under the configured module
  roots.
- `default-modules.yaml` and the plugin registry under the configuration root.
- Authored specification documents and the Quire engine's structured JSON
  contract.
- The caller-selected repository root and configuration root.

## Outputs

- One assembled catalog per configuration root.
- Authoring contracts, validation verdicts and completeness findings as
  versioned boundary result documents.
- Materialized module roots and registry records under the configuration root.

## Behavior

- `quoin-quire` SHALL wrap `quire-rs` as a Cargo dependency rather than
  re-implementing Quire behaviour, and SHALL replace the vendored Quire contract
  machinery in `src/quire/`.
- `quoin-validators` SHALL evaluate the validator behaviour currently in
  `src/validators/` and SHALL be the first capability cut over, so that the
  boundary, the error taxonomy, the generated type surface, the differential
  harness and the deletion step are all exercised on a bounded surface.
- `quoin-semantic` and `quoin-completeness` SHALL evaluate semantic manifest
  validation and completeness analysis, preserving the existing verdicts.
- `quoin-config` SHALL resolve the configuration root and the authoring
  organization, and `quoin-modules` SHALL locate, reconcile, deduplicate and
  materialize module roots, using `gix` for git access.
- `quoin-catalog` SHALL assemble one catalog from the resolved module roots,
  SHALL resolve a requested type case-insensitively, and SHALL report duplicate
  type declarations rather than silently selecting one.
- Quoin SHALL reimplement natively the nine `@agent-ix/ix-cli-core` behaviours it
  uses and SHALL NOT port that package's authentication, secrets or marketplace
  surface.
- Module reconcile SHALL remain idempotent and SHALL perform no git or network
  access when every declared module is already materialized at its pinned
  reference.
- If a module manifest declares an unknown key, then the Rust manifest reader
  SHALL refuse it, preserving the existing strict-parsing behaviour.
- Quoin SHALL keep exactly one catalog and one module store across the port and
  SHALL NOT introduce a second catalog, registry or module resolver.
- Markdown skill prose outside `workflow-assets/` SHALL remain data; the Rust
  implementation SHALL resolve the skills root from an argument or environment
  value with an embedded fallback, and SHALL carry the existing
  skill-vocabulary-drift and skill-contract assertions as Rust tests reading the
  same Markdown.
- Quoin SHALL classify `skills/**/workflow-assets/**` as executable assertion
  logic rather than as inert data, because `src/flows.ts:57` spawns ix-flow
  against those assets and their `specInvariants` decide whether a review, matrix
  or plan flow passes, and SHALL record one disposition for each such asset.

## Error Conditions

An unreadable or strictly invalid manifest, a duplicate type declaration, an
unresolvable module reference, a missing module root, an unavailable Quire
engine, and a git reference that does not resolve each produce a distinguishable
refusal and are never reported as an empty but successful catalog.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-099-CON-1 | Quoin SHALL NOT create a second catalog, registry or module resolver. | Architecture | Test |
| FR-099-CON-2 | `quoin-quire` SHALL NOT re-implement Quire parsing, extraction or validation semantics. | Responsibility | Inspection |
| FR-099-CON-3 | Reconcile SHALL NOT perform network access when the declared modules are already materialized. | Behavior | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-099-AC-1 | `quoin-core validators.run` returns the same verdict and non-success classification as the retained validator path over the golden corpus. | Test (TC-1625) |
| FR-099-AC-2 | `quoin-core quire.validate` returns the same verdict as the retained `src/quire` adapter for every corpus document, with `quire-rs` consumed as a Cargo dependency. | Test (TC-1626) |
| FR-099-AC-3 | `quoin-core catalog.list` assembles a catalog identical to the retained catalog for the same configuration root, resolves a type case-insensitively, and reports a planted duplicate type rather than selecting one. | Test (TC-1627) |
| FR-099-AC-4 | A second reconcile over an already-materialized configuration root performs no git or network access and yields a catalog identical to the first. | Test (TC-1628) |
| FR-099-AC-5 | A manifest carrying an unknown key is refused by the Rust reader with the same classification as the retained reader. | Test (TC-1629) |
| FR-099-AC-6 | `quoin-core semantic.validate` and `quoin-core completeness.analyze` return verdicts identical to the retained implementations over the golden corpus. | Test (TC-1630) |
| FR-099-AC-7 | The Rust skill tests read the shipped Markdown skills and fail on a planted vocabulary drift, and the binary resolves the skills root from an argument, an environment value and its embedded fallback. | Test (TC-1631) |
| FR-099-AC-8 | The workspace declares no dependency on `@agent-ix/ix-cli-core`, and each of the nine behaviours it supplied has a named Rust implementation and a test. | Test (TC-1632) |
| FR-099-AC-9 | Every asset under `skills/**/workflow-assets/**` is classified as executable assertion logic with a recorded disposition, and an unclassified asset fails the gate. | Test (TC-1683) |

## Dependencies

- **Upstream**: [FR-096](./FR-096-versioned-rust-engine-boundary.md) and [FR-098](./FR-098-semantic-and-identity-parity.md); [FR-007](./FR-007-assemble-module-roots.md), [FR-017](./FR-017-reconcile-default-modules.md) and [FR-029](./FR-029-consume-the-quire-json-contract.md), whose behaviour it preserves.
- **Downstream**: [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md) and [FR-102](./FR-102-command-surface-and-oclif-retirement.md).
