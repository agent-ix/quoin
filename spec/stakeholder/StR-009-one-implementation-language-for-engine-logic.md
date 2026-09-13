---
id: StR-009
title: "Owners need one implementation language and one schema source for Quoin engine logic"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-096"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-097"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-098"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-099"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-100"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-101"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-102"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-103"
    type: "satisfied_by"
---

# StR-009: Owners need one implementation language and one schema source for Quoin engine logic

## Stakeholder Need

The repository owner requires that Quoin's first-party engine, production,
planning, validation, canonicalization, digest, oracle, evidence and
qualification behaviour shall be implemented by the Quoin Rust workspace behind
one versioned boundary, and that Quoin shall source every type crossing that
boundary from one schema source, so that Quoin's implementation language, ownership, compatibility and
test evidence are explicit rather than inferred from whichever file was edited
last.

## Rationale

Quoin's first-party behaviour is currently divided across 105,814 physical lines
in 364 in-repository files, measured at `e718d45` over `git ls-files`
intersected with `{.ts, .mjs, .js, .py, .sh, .tsp}`. The largest single block is
`skills/` at 23,164 lines, whose `workflow-assets` are executable rather than
inert because `src/flows.ts:57` spawns ix-flow against them and their
`specInvariants` decide whether a review, matrix or plan flow passes. The
`corpus/` tree is a git submodule pointing at `agent-ix/qa-corpus` and is not
counted here, because it is another repository's tree. The campaign
implementation language policy requires first-party production and
qualification-path implementation to be Rust; Amendment 1, owner directive
2026-09-12, withdraws Quoin's earlier exemption and directs a staged port.

`engineering-assurance` completed the same transition, which is why this need is
stated as a language and schema-source need rather than as a rewrite. The port
relocates Quoin's implementation language. It does not relocate ownership: Quoin
remains the sole owner of its evidence store, its catalog and its measurement
model, and Quire, `filament-core-data`, `engineering-assurance`, ix-flow and the
module repositories keep their own domains.

The schema-source half of the need is separate from the language half and is
easy to lose. A Rust engine that hand-writes its own copy of a type
`filament-core-data` publishes has moved the duplication rather than removed it,
and it does so invisibly, because both copies compile.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-009-VC-1 | Quoin's first-party engine logic is reached through one versioned Rust boundary hosted in this repository; no additional repository owns the port, demonstrated by building and invoking `quoin-core` from a clean checkout of this repository alone. | Demonstration |
| StR-009-VC-2 | Every first-party executable path in this repository carries one current state and one final disposition — port, retain as approved TypeScript, retain as data, or delete — with no unclassified path. | Test (TC-1601) |
| StR-009-VC-3 | Differential qualification preserves accepted success, non-success, malformed, refusal, canonicalization and digest behaviour across each cutover. | Test (TC-1602) |
| StR-009-VC-4 | No type crossing the boundary is hand-written where `filament-core-data` or a repository schema publishes it, and every generated type records its generator provenance. | Test (TC-1603) |
| StR-009-VC-5 | Completion evidence identifies no unapproved first-party non-Rust engine, production, planning, validation, canonicalization, digest, oracle or assertion logic, and the check that produces it is demonstrated to fail on a planted violation. | Test (TC-1604) |

## Stakeholders

The primary stakeholder is the repository owner, who is accountable for the
implementation-language boundary across the campaign and who issued Amendment 1.
Affected parties are Quoin maintainers, who carry the staged cutovers;
`filament-core-data`, which publishes the schema-sourced types this need depends
on; `engineering-assurance`, from which shared assurance capability is consumed;
and every consumer of Quoin's command surface, whose behaviour must not change
until the surface deliberately changes.

## Context and Assumptions

Two TypeScript categories are standing-approved by Amendment 1 and are **not**
program debt: user-interface code, and types and clients published by
`filament-core-data`. This need does not ask for either to be removed.

It is assumed that `filament-core-data` reaches its cross-language compatibility
gate before type retirement begins, that `quire-rs` remains the Quire engine the
adapter wraps, and that shared assurance capability is consumed from
`engineering-assurance` rather than regrown. It is assumed that the existing
TypeScript test suite remains runnable as the parity oracle until each
capability cuts over.

## Stakeholder Constraints (Contextual)

The owner has stated that no evidence byte and no accepted-corpus byte changes
across the port; that coexistence during a staged cutover is not a violation and
is not reportable as completed remediation; and that no crate publishes to
crates.io and no package publishes to public npmjs while this work runs.

## Dependencies

**Upstream**: the campaign implementation language policy as amended
2026-09-12, and
[ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md),
which records the boundary, the crate topology and the capability dispositions.
**Downstream**: the functional requirements
[FR-096](../functional/FR-096-versioned-rust-engine-boundary.md) through
[FR-103](../functional/FR-103-corpus-consolidation.md) and the constraints
[NFR-024](../non-functional/NFR-024-bounded-staged-coexistence.md) through
[NFR-027](../non-functional/NFR-027-rust-implementation-idioms-and-gates.md).

## Priority and Risk (Informative)

P0 for the campaign. The risk if unmet is that Quoin remains the largest
first-party non-Rust surface in the ecosystem while every repository around it
converges, so the policy cannot be asserted anywhere. The risk of doing it badly
is higher than of not doing it: a port that changes a digest, forks the evidence
store, or reports coexistence as remediation destroys the evidence the programme
exists to produce.

## Traceability

This need sits beside [StR-007](./StR-007-governed-assurance-portfolio.md), which
owns the assurance portfolio Quoin publishes, and beside
[StR-003](./StR-003-shared-catalog.md), which owns the single catalog. Neither is
changed by this need; both constrain it, because a port must not fork what they
require to be singular. It is the Quoin sibling of
`ix://agent-ix/engineering-assurance/StR-003`.
