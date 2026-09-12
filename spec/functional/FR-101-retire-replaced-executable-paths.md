---
id: FR-101
title: "Retire replaced Quoin executable paths"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "implements"
  - target: "ix://agent-ix/quoin/US-024"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-096"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-098"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-099"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-100"
    type: "requires"
---

# FR-101: Retire replaced Quoin executable paths

## Description

Quoin SHALL retain each replaced TypeScript, JavaScript, MJS or Python
implementation until its Rust replacement is demonstrated equivalent at one
candidate revision, then cut over the direct invocation in a reversible change,
and only afterwards delete the replaced path together with its tests.

## Inputs

- The executable-path disposition matrix at
  `docs/rust-burndown/executable-path-matrix.md`, whose dispositions follow
  [ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md).
- The allowance manifest `.language-allowances.yaml` at the repository root.
- The capability-gap matrix owned by quire-research LR03.
- The reviewed Rust crates and the `quoin-core` boundary.
- Same-revision differential, digest-replay, property and command-surface
  evidence.

## Outputs

- One disposition for each first-party executable path in this repository: port,
  retain as approved TypeScript, retain as data, or delete.
- A reversible cutover change per capability, separate from its deletion.
- A final inventory of retained configuration and inert foreign-language data.

## Behavior

- Quoin SHALL follow the port and cutover order in ADR-0003.
- Quoin SHALL keep a replaced path until its Rust replacement passes locally at
  the same candidate revision across differential, property and, for
  store-backed capabilities, digest-replay evidence.
- Quoin SHALL record the cutover of a capability's direct invocation as its own
  change, separate from the change that deletes the replaced path, and SHALL
  demonstrate that reverting the cutover restores the retained invocation.
- Quoin SHALL delete a replaced implementation and the tests covering it in the
  same change, and deletion SHALL be the final step for that capability.
- When a retained test is to be deleted, Quoin SHALL first restate every
  acceptance criterion that test carried on a tracking-tagged Rust test that
  fails when its property is violated.
- Quoin SHALL replace every differential that executes the retained TypeScript,
  JavaScript, MJS or Python as an oracle with expected fixtures captured once
  from the retained implementation and committed to this repository, so that no
  non-Rust implementation is executed as a test oracle after its capability's
  cutover.
- Each committed expected fixture SHALL record the implementation that produced
  it, that implementation's revision, and the digest of the request that
  produced it.
- A fixture recorded as produced by the Rust implementation SHALL NOT satisfy a
  parity criterion, because a fixture captured from the implementation under
  test is the self-written-fixture failure raised one level.
- While a criterion a retained path carried is backed only by a test that
  asserts against a fixture that test, its setup helper or its own generation
  step wrote, or by a check that passes over an empty population, Quoin SHALL NOT
  record the removal as complete.
- Where a retired test carries a criterion with no native replacement, Quoin
  SHALL relocate that test rather than delete it, and SHALL record which
  criterion it carries.
- Quoin SHALL classify user-interface TypeScript and `filament-core-data`-published
  types and clients as retained approved code, and SHALL NOT record a
  disposition of delete against either.
- Quoin SHALL classify a foreign-language sample input that determines no
  assertion as inert data through an entry in the allowance manifest, and SHALL
  exclude it from executable debt.
- Quoin SHALL classify `skills/**/workflow-assets/**` as executable assertion
  logic rather than as inert data, because `src/flows.ts:57` spawns ix-flow
  against those assets and their `specInvariants` decide whether a review, matrix
  or plan flow passes.
- Quoin SHALL record no port or delete disposition for a path inside the
  `corpus/` submodule, which belongs to `agent-ix/qa-corpus`.
- Quoin SHALL leave every evidence byte and every accepted-corpus byte unchanged
  through cutover, revert and deletion.
- Quoin SHALL return a changed interface or compatibility promise to
  specification before implementation continues.

## Error Conditions

Missing parity evidence, mismatched candidate revisions, a differential whose
compared population is empty, a criterion backed only by a self-written fixture,
a changed evidence or corpus byte, a test deleted without a restated criterion,
and an unresolved path disposition each block removal.

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-101-CON-1 | Deletion SHALL be the final step for each replaced capability. | Lifecycle | Test |
| FR-101-CON-2 | Cutover SHALL NOT rewrite historical evidence or accepted-corpus bytes. | Data Integrity | Test |
| FR-101-CON-3 | A criterion backed only by a self-written fixture or by a check over an empty population SHALL NOT satisfy a removal. | Reporting | Test |
| FR-101-CON-4 | No non-Rust implementation SHALL execute as a test oracle after its capability's cutover. | Verification | Test |
| FR-101-CON-5 | A standing-approved TypeScript category SHALL NOT be recorded with a disposition of delete. | Scope | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-101-AC-1 | Every first-party executable-path row records one current state and one final disposition, and the gate fails when a row is unclassified. | Test (TC-1641) |
| FR-101-AC-2 | Removal is refused unless the retained and Rust paths both pass at the same candidate revision and the direct invocation already dispatches to the Rust interface. | Property (TC-1642) |
| FR-101-AC-3 | Reverting a cutover change restores the retained invocation, and comparing the evidence and accepted-corpus trees before and after the revert finds zero changed bytes. | Test (TC-1643) |
| FR-101-AC-4 | For each deleted test, the criterion it carried resolves to a tracking-tagged Rust test, and mutating the corresponding Rust source makes that test fail. | Test (TC-1644) |
| FR-101-AC-5 | After a capability's cutover, no test in the repository spawns or imports a non-Rust implementation of that capability as an oracle; the expected fixtures are committed. | Test (TC-1645) |
| FR-101-AC-6 | `quoin-core lint.removal` refuses a removal justified by a check whose reported population is zero, or by a test whose asserted fixture was written by that test, its setup helper or its own generation step. | Test (TC-1646) |
| FR-101-AC-7 | `make lint` and `make test` pass at the candidate revision with no observable behaviour change recorded by the command-surface snapshot. | Test (TC-1647) |
| FR-101-AC-8 | The final inventory and `quoin-core lint.language` report no unapproved first-party non-Rust engine, production, planning, validation, canonicalization, digest, oracle or assertion logic, exclude manifest-declared inert samples from executable debt, and fail on a planted violation. | Test (TC-1648) |
| FR-101-AC-9 | Run against the baseline revision `e718d45`, the path inventory reports 105,814 physical lines across 364 in-repository files, including `skills/` and `bin/`, and excludes every path inside the `corpus/` submodule; the figures are asserted of that revision, not of the revision under test. | Test (TC-1690) |
| FR-101-AC-10 | The inventory population includes executable paths with no governed extension — Makefile recipes and `run:` blocks under `.github/workflows/**` — and a planted non-Rust assertion in one of them is reported. | Test (TC-1700) |
| FR-101-AC-11 | Each committed expected fixture records its producing implementation, that implementation's revision and the request digest, and a fixture recorded as produced by the Rust implementation fails the parity gate. | Test (TC-1701) |
| FR-101-AC-12 | A retained path is identified by a stable identity that survives a rename, so moving a file does not silently drop its retention row, its successor reference or its expiry. | Test (TC-1702) |

## Dependencies

- **Upstream**: accepted [ADR-0003](../../docs/semantic-module-architecture/adr/0003-rust-native-quoin-engine-boundary.md); completed [FR-096](./FR-096-versioned-rust-engine-boundary.md) through [FR-100](./FR-100-rust-evidence-measurement-change-assurance.md), which own the dependency-cycle break; [NFR-024](../non-functional/NFR-024-bounded-staged-coexistence.md), which defines the allowance manifest and the successor reference this requirement records against; quire-research LR03, which owns the shared capability-gap matrix.
- **Downstream**: [NFR-024](../non-functional/NFR-024-bounded-staged-coexistence.md) bounds the retention this requirement creates; [NFR-025](../non-functional/NFR-025-immutable-evidence-and-corpus-bytes.md) constrains its byte guarantee.
