---
id: SR-153
title: "Integrity review of the Rust burn-down spec set"
type: SpecReview
analysis: integrity
scope: "ADR-0003, StR-009, US-024, FR-096..FR-103, NFR-024..NFR-027, IT-003"
review_set: all
---

# Integrity review of the Rust burn-down spec set

## Summary

This analysis checked the fifteen artefacts of EPIC #373 — ADR-0003, StR-009,
US-024, FR-096 through FR-103, NFR-024 through NFR-027 and IT-003 — for
completeness of traceability, internal consistency and conflict, atomicity and
testability of each obligation, and the hidden assumptions that external
commands, multi-source lookups and authenticated registries carry. Coverage of
the vertical chain is sound: every FR traces to StR-009, StR-009 names all eight
as `satisfied_by`, and every acceptance criterion but two carries a verification
method. Twenty findings are recorded: three high — a circular delivery ordering
between FR-103, FR-101 and FR-100 that also contradicts ADR-0003, an allowance
numbering the ADR and NFR-024 disagree on and that no artefact defines, and an
NFR-024 successor rule that the ADR itself records as unsatisfiable on the day
it lands — eleven medium and six low.

## Traceability

| Chain | Result | Evidence |
|---|---|---|
| US -> FR/SR | Partial | US-024 drives the set in prose; only FR-096 declares `implements US-024` (FND-018) |
| FR/SR -> StR | Pass | FR-096..FR-103 each declare `implements` StR-009; StR-009 lists all eight as `satisfied_by` |
| FR/SR -> verification | Partial | 91 of 93 criteria name a TC identifier; FR-097-AC-7 and FR-102-AC-7 name only Inspection (FND-016) |
| NFR scoped and referenced | Partial | NFR-024..NFR-027 each declare Scope and constrain named FRs, but FR-102 and FR-103 are reached by no Rust-idiom or toolchain relationship (FND-006), and NFR-021..NFR-023 are not extended to the ported measurement crates (FND-007) |
| IT -> FR | Pass | IT-003 `verifies` FR-097 and FR-096, and both name it downstream |
| ADR -> requirements | Pass | ADR-0003 frontmatter lists StR-009, FR-096..FR-103 and NFR-024..NFR-027; each requirement cites it upstream |

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                                 | Refs                                          |
| ------- | -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| FND-001 | high     | FR-103 declares FR-101 upstream, FR-101 requires FR-096..FR-100 completed, and FR-103's Behavior requires consolidation to complete before the measurement crates are ported while naming FR-100 downstream — a cycle; ADR-0003 instead says corpus consolidation runs in parallel and independently. | FR-103, FR-101, FR-100, ADR-0003              |
| FND-002 | high     | The allowance set is undefined and inconsistently numbered: ADR-0003 relies on "allowance 5" for retained-path successors and "metric allowance 3" for inert samples, while NFR-024 bounds the manifest to allowance numbers 1 through 4.                                                            | NFR-024, ADR-0003, FR-101, FR-103             |
| FND-003 | high     | NFR-024 defines a valid successor as an open sub-issue of the burn-down epic, ADR-0003 records that delivery-stage tickets 0 through 9 do not exist, and no requirement in the set owns creating them — so the first enforcement run fails the whole retained surface with no stated transition.      | NFR-024, ADR-0003                             |
| FND-004 | medium   | The allowance manifest is given two paths: NFR-024's Verification names `quoin/.language-allowances.yaml` while calling it checked in at the repository root, and NFR-024-AC-6 asserts `.language-allowances.yaml` at the repository root.                                                            | NFR-024, NFR-024-AC-6                         |
| FND-005 | medium   | The clause requiring a changed interface or compatibility promise to be returned to specification appears in both FR-096 and FR-101 in different wording, is not externally observable, and carries no acceptance criterion in either.                                                               | FR-096, FR-101                                |
| FND-006 | medium   | NFR-027's Scope claims every crate in the `rust/` workspace but its relationships reach only FR-096, FR-099, FR-100 and FR-101, and NFR-026 only FR-096, FR-099 and FR-100 — leaving `quoin-cli` (FR-102) and the FR-103 consolidation crates unconstrained by relationship.                          | NFR-026, NFR-027, FR-102, FR-103              |
| FND-007 | medium   | FR-100 preserves the behaviour of FR-084, FR-085, FR-090 and FR-092 but carries no reproducibility, wall-clock or figure-provenance criterion, and no relationship extends NFR-021, NFR-022 or NFR-023 to the five Rust measurement crates.                                                          | FR-100, NFR-021, NFR-022, NFR-023             |
| FND-008 | medium   | FR-097 admits two schema sources — `filament-core-data`-published types and repository-owned `quoin-schemas` definitions — and states no tie-breaking policy when both publish the same type, where FR-099 states one for duplicate catalog types.                                                   | FR-097, FR-099                                |
| FND-009 | medium   | No requirement declares a floor, detection method or user-facing error for the external tools and authenticated registries this set depends on — git via `gix`, submodule pinning, `cargo fetch`, `pnpm install --frozen-lockfile` and `npm.ix` credentials; NFR-020's scope lists none of them and NFR-026 covers only the Rust channel. | FR-099, FR-103, IT-003, NFR-020, NFR-026      |
| FND-010 | medium   | Generator provenance has two homes with no stated precedence: FR-097 requires a provenance record per generated artefact, and NFR-024-AC-7 requires each allowance-2 manifest entry to declare generator identity and source-schema digest.                                                          | FR-097, NFR-024-AC-7                          |
| FND-011 | medium   | The enforcement run's output is owned twice: FR-100 requires it to write the first-party non-Rust line count into the evidence store, and NFR-024's Verification requires the same run to write three classification counts; neither references the other.                                            | FR-100, NFR-024                               |
| FND-012 | medium   | NFR-024 requires an expiry date but states no maximum retention period and permits an unbounded re-dated owner decision, so a retention can be renewed indefinitely while satisfying every criterion the requirement carries.                                                                        | NFR-024-AC-1, NFR-024-AC-2                    |
| FND-013 | medium   | The Rust toolchain floor is simultaneously decided and open: NFR-026 fixes the channel at 1.98.1 and makes it a gate, while ADR-0003 lists the 1.94.1 pins of `quire-rs` and `filament-core-data` as an open question for the owner.                                                                 | NFR-026, ADR-0003                             |
| FND-014 | medium   | FR-098's differential harness executes the retained TypeScript as an oracle, which FR-101-CON-4 forbids after a capability's cutover; FR-098 states no retirement point for its own harness and no criterion covers the move to committed fixtures.                                                  | FR-098, FR-101                                |
| FND-015 | medium   | NFR-026-AC-3 fails the gate on a second declaration of the channel "anywhere in the repository", which collides with NFR-027's own scope exclusion for inert Rust sample inputs under the corpus and with vendored trees the repository does not author.                                             | NFR-026-AC-3, NFR-027                         |
| FND-016 | low      | FR-097-AC-7 and FR-102-AC-7 are the only acceptance criteria in the batch with no TC identifier, naming Inspection alone, so the Test Matrix has no row identity to bind them to.                                                                                                                    | FR-097-AC-7, FR-102-AC-7                      |
| FND-017 | low      | Measured constants are embedded in criteria without the revision that produced them: FR-101-AC-9 asserts 105,814 lines across 364 files and FR-102 asserts 59 declared commands, while `e718d45` appears only in prose.                                                                              | FR-101-AC-9, FR-102                           |
| FND-018 | low      | US-024 traces only to StR-009 and only FR-096 declares `implements US-024`, so the story's five acceptance examples reach FR-097 through FR-103 through prose rather than through a declared edge.                                                                                                   | US-024, FR-097, FR-103                        |
| FND-019 | low      | Three overlapping corpus terms are used and none is defined in the set — "golden corpora" in FR-098, "governed corpus" in FR-100 and FR-103, "accepted corpus" in FR-101 and NFR-025 — yet the digest-immutability criteria depend on which population is meant.                                     | FR-098, FR-100, FR-101, FR-103, NFR-025       |
| FND-020 | low      | Several criteria bundle independent obligations, so a failure does not identify which broke: FR-096-AC-6 pairs the 67,108,864-byte payload with the three-way termination taxonomy, FR-099-AC-3 carries catalog identity, case-insensitive resolution and duplicate reporting, and NFR-024-AC-2 carries three planted scenarios. | FR-096-AC-6, FR-099-AC-3, NFR-024-AC-2        |

## Consistency and Conflict

Beyond the contradictions recorded above, the set is internally consistent on
the points it repeats most often. The empty-population rule is stated
identically in FR-098, FR-101, NFR-024, NFR-025 and NFR-027 — an inconclusive
result is never a clean one. The registry restriction is stated in FR-096,
FR-097, FR-102 and FR-103 in compatible terms. The two standing-approved
TypeScript categories are named the same way in ADR-0003, StR-009, US-024,
FR-097, FR-101 and NFR-024. The disposition of `skills/**/workflow-assets/**`
is stated in near-identical wording by FR-099 and FR-101 with an acceptance
criterion only in FR-099-AC-9, while ADR-0003 defers the disposition itself to
delivery stage 8; the duplication is benign because the two statements agree,
but only FR-099 is enforceable.

## Atomicity and Testability

Every FR Behavior bullet but one states a single externally observable
obligation, and the criteria overwhelmingly name a command, a planted violation
or a counted population rather than an intent. The exceptions are recorded as
FND-005, which is untestable as written, and FND-020, which is testable but not
atomic. Nothing in the set requires rewriting for lack of an observable.

## Failure Domain Cross-check

Extension failures, identity keys and evaluation purity are covered: the oclif
extension contract is dispositioned by FR-102, digest identity is the subject of
FR-098 and NFR-025, and read-only behaviour over the governed corpus is asserted
by FR-100-CON-3 and FR-100-AC-5. The topological gap is FND-001, where the
declared dependency edges between FR-100, FR-101 and FR-103 do not form an
executable order.
