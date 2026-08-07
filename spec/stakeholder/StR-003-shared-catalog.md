---
id: StR-003
title: "Authoring and validation share one consistent catalog"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-007"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-014"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-017"
    type: "satisfied_by"
---

# StR-003: Authoring and validation share one consistent catalog

## Stakeholder Need

Authors require that the type vocabulary they author against and the vocabulary
their work is validated against shall be one and the same catalog, assembled from
a single shared module store, so that a contract `quoin` hands them always matches
the rules `quire` later enforces.

## Rationale

Spec authoring is a write-then-validate loop. If the authoring tool drew its
skeletons and schemas from a different place than the validator read its rules,
an author could faithfully follow a contract and still fail validation — a
confusing, trust-eroding outcome. Pointing both at the same `~/.ix/filament/modules`
store guarantees the contract the author receives is the contract that is
enforced, so a clean authoring pass predicts a clean validation pass.

## Validation Criteria


| ID | Criteria | Validation |
|----|----------|------------|
| StR-003-VC-1 | The modules `quoin` reads to build an authoring contract are the same modules `quire` reads to validate, such that a file authored to the returned skeleton and schema validates without rule mismatch. | Demonstration |

Satisfaction is demonstrated by authoring an artifact from a `quoin` contract and validating it with `quire` against the same store.

## Stakeholders

The primary stakeholders are spec-authoring agents; affected parties are
reviewers and downstream tools that rely on validated, consistent specs.

## Dependencies

**Downstream**: the catalog-assembly, authoring-contract, and reconcile functional
requirements ([FR-007](../functional/FR-007-assemble-module-roots.md),
[FR-014](../functional/FR-014-emit-authoring-contract.md),
[FR-017](../functional/FR-017-reconcile-default-modules.md)). The shared store is
also read by `quire`.
