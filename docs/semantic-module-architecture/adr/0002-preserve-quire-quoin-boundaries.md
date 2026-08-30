---
id: ARCH-SM-ADR-0002
title: "Preserve Quire and Quoin boundaries while adding semantic packages"
status: proposed
date: 2026-08-29
requirements:
  - FR-047
  - FR-050
---

# ADR-0002: Preserve Quire and Quoin boundaries while adding semantic packages

## Context

The semantic-data program introduces a shared kernel, IR, compatibility comparison, compiler,
emitters, and generated packages. Quire and Quoin already have deliberate scopes. Adding a new
compiler concern must not revive removed rendering behavior or absorb catalog, parser, workflow,
consumer, and persistence responsibilities into one meta-system implementation.

## Proposed decision

Preserve the existing roles and add compiler/package work as a separate owner:

- Quire parses, validates, extracts, addresses, and byte-splices typed Markdown using module
  contracts.
- Quoin discovers and distributes modules, maintains locks/install/update, exposes authoring
  contracts, and orchestrates skills, workflows, advice, generation proposals, and evidence audits.
- `filament-core-data` or its separately governed reusable compiler product owns semantic IR,
  compatibility rules, compiler behavior, emitters, and generated package contracts.
- Modules own domain vocabulary, mappings, constraints, examples, and semantic versions.
- Consumers own adapters, persistence, migrations, runtime state, API/IPC choices, and presentation.

Quire does not become a renderer or cross-language generator. Quoin does not become the parser or
semantic compiler. Consumer CI remains the executor of L2 verification.

## Quire decision compatibility

- ADR-0002's byte-oriented pipeline is retained, while its draft rendering allocation is historical
  under the current render-removed specification.
- ADR-0003's unified archetype shape is retained as a structural parser model, not promoted into a
  universal semantic base class.
- ADR-0004's direct Markdown direction is retained and clarified by the current document-boundary
  canonical Markdown contract.
- Accepted ADR-0011 remains governing for validation levels and capability roles.

## Consequences

- Quire may return generic JSON/open values for dynamically discovered types without promising
  native generated packages for every module.
- A finite consumer may adopt native packages without closing Quoin/Quire's dynamic module model.
- Compiler and publication changes have their own repositories, owners, conformance corpus,
  compatibility policy, release qualification, and incident response.
- An ownership move requires a successor ADR in the owning repository; code dependency drift does
  not rewrite this decision.

## Alternatives considered

- **Put cross-language generation in Quire:** rejected because it reverses rendering removal and
  couples the parser to package publication.
- **Put parsing and the compiler in Quoin:** rejected because it duplicates Quire semantics and
  mixes catalog/workflow orchestration with document mechanics.
- **Allow each module to implement its own compiler:** rejected because shared compatibility and
  cross-language conformance would fragment.
- **Require generated packages for every installed module:** rejected because it closes the dynamic
  ecosystem and forces unrelated consumers to regenerate.

## Promotion

Status remains proposed until named Quoin/Quire maintainers review the issue #289 PR. This ADR does
not change external Quire ADR status; any external normative edit belongs in `quire-rs`.
