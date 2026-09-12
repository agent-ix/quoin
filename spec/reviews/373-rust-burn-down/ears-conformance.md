---
id: SR-158
title: "EARS-conformance review of the Rust burn-down spec set"
type: SpecReview
analysis: ears-conformance
scope: "StR-009, FR-096..FR-103, NFR-024..NFR-027"
review_set: all
---

# EARS-conformance review of the Rust burn-down spec set

## Summary

The engine grammar check (`quire validate`, `[ears:*]` warnings) was run over
each of the thirteen artifacts in the Rust burn-down batch — StR-009,
FR-096..FR-103 and NFR-024..NFR-027 — and reported 43 warnings across 9 of the
13 artifacts: 14 `non-singular`, 13 `unclassifiable`, 13 `missing-subject`,
3 `non-canonical-trigger` and 0 `vague-response`. StR-009, NFR-024, NFR-025 and
NFR-026 are engine-clean. Every `unclassifiable` hit is co-located with a
`missing-subject` hit on the same line, and all 13 such pairs fall on a
*continuation* line of a wrapped bullet: the engine splits statements on
newlines, so the subject sits on the preceding line. Ten of those 13 pairs are
the visible half of a conjoined `… SHALL … and SHALL …` bullet — i.e. the real
defect is `non-singular`, under-counted by the engine — and three (FR-096:62,
FR-099:63, FR-100:49-in-part) are pure wrapping artifacts over statements that
are otherwise well-formed. The dominant genuine defect is therefore
**non-singularity**: multi-obligation bullets that cannot map one-to-one onto an
acceptance criterion. Semantic review adds four statements the engine missed for
the same reason (NFR-024, NFR-026, StR-009 and a family of wrapped bullets in
FR-100/FR-101/FR-103), plus two statements that bury an unwanted-condition
(`if … then …`) inside a ubiquitous one. No statement in the batch trips the
vague-verb lexicon, and no ambiguity found here changes what gets built, so
nothing is raised to high.

## Findings

| ID      | Severity | Summary                                                                                                                                                      | Refs    |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------- |
| FND-001 | medium   | FR-096:45-47 conjoins `SHALL expose only …` with `SHALL NOT expose …`; engine sees the wrapped half as `unclassifiable` + `missing-subject` at FR-096:46 — split into two requirements | FR-096  |
| FND-002 | low      | FR-096:61-62 `No Node runtime type … SHALL appear …` is a single well-formed prohibition; the `unclassifiable` + `missing-subject` pair at FR-096:62 is a line-wrap artifact, but the negative-subject form reads better as `Quoin SHALL NOT place … in a boundary request or result document` | FR-096  |
| FND-003 | medium   | FR-097:52-55 conjoins a retirement precondition with a replacement obligation; engine reports `unclassifiable` + `missing-subject` at FR-097:54 — split, and state the precondition as `Until filament-core-data publishes the replacing package, …` | FR-097  |
| FND-004 | medium   | FR-097:59-61 `SHALL remain approved target-language code and SHALL NOT be reported as duplicates` is non-singular (engine: `non-singular` + `unclassifiable` + `missing-subject` at FR-097:60) | FR-097  |
| FND-005 | medium   | FR-098:44 packs two `SHALL treat` obligations (verdict contractual, diagnostic text non-contractual) into one statement — `non-singular` | FR-098  |
| FND-006 | low      | FR-098:53 leads with `Before a store-backed capability cuts over, …` — `non-canonical-trigger`; use `When a store-backed capability is about to cut over, …` | FR-098  |
| FND-007 | medium   | FR-098:53-55 also conjoins the replay obligation with the refusal obligation; engine reports `unclassifiable` + `missing-subject` at FR-098:55 — the refusal belongs in its own `If any digest differs, then …` requirement | FR-098  |
| FND-008 | medium   | FR-098:60-62 `SHALL refuse the cutover and SHALL name the differing input …` is non-singular inside an otherwise correct unwanted-behaviour pattern (engine: `non-singular` at FR-098:61) | FR-098  |
| FND-009 | medium   | FR-099:53-55 chains three obligations on `quoin-catalog` (assemble, resolve case-insensitively, report duplicates); engine reports `non-singular` + `unclassifiable` + `missing-subject` at FR-099:54 | FR-099  |
| FND-010 | medium   | FR-099:56-58 conjoins `SHALL reimplement … nine behaviours` with `SHALL NOT port … authentication, secrets or marketplace surface` — split the obligation from the exclusion | FR-099  |
| FND-011 | medium   | FR-099:59 `Module reconcile SHALL remain idempotent and SHALL perform no git or network access …` is non-singular, and its subject is a process name rather than a named system/actor | FR-099  |
| FND-012 | low      | FR-099:62-63 is a correct `If … then …` unwanted-behaviour statement; the `unclassifiable` + `missing-subject` pair at FR-099:63 is a line-wrap artifact only | FR-099  |
| FND-013 | medium   | FR-099:64-65 `SHALL keep exactly one catalog … and SHALL NOT introduce a second …` is non-singular (engine: `unclassifiable` + `missing-subject` at FR-099:65) | FR-099  |
| FND-014 | medium   | FR-100:47-49 conjoins `quoin-store SHALL own …` with `every other crate SHALL obtain them from it` — two subjects, two obligations (engine: `unclassifiable` + `missing-subject` at FR-100:49) | FR-100  |
| FND-015 | medium   | FR-100:63 `SHALL remain advisory and read-only … and SHALL write no corpus byte` is non-singular | FR-100  |
| FND-016 | medium   | FR-100:75 `The evidence store SHALL remain Quoin's, and Quoin SHALL NOT take a dependency on engineering-assurance …` is non-singular with two distinct subjects | FR-100  |
| FND-017 | low      | FR-101:54 leads with `Before deleting a retained test, …` — `non-canonical-trigger`; use `When a retained test is to be deleted, …` | FR-101  |
| FND-018 | medium   | FR-101:65-67 `SHALL relocate that test … and SHALL record which criterion it carries` is non-singular (engine: `non-singular` + `unclassifiable` + `missing-subject` at FR-101:66) | FR-101  |
| FND-019 | low      | FR-102:41-42 `… through every stage before the shell replacement, and SHALL treat any snapshot difference as a regression` trips `non-canonical-trigger` on `before` at FR-102:42; here `before` is a temporal qualifier, not a trigger — the reportable defect is the conjoined second obligation | FR-102  |
| FND-020 | medium   | FR-102:59-60 `quoin-cli SHALL refuse it … and SHALL name the unrecognised command` is non-singular inside a correct `If … then …` pattern | FR-102  |
| FND-021 | medium   | FR-102:62-64 `quoin-core SHALL become quoin, and Quoin SHALL delete src/quire/exec.ts and src/core/exec.ts …` is non-singular with two subjects | FR-102  |
| FND-022 | medium   | FR-102:65-67 chains three publication obligations (`SHALL publish … to npm.ix`, `SHALL NOT publish … to public npmjs`, `SHALL NOT publish quoin-cli to crates.io`); engine reports `non-singular` + `unclassifiable` + `missing-subject` at FR-102:66 | FR-102  |
| FND-023 | medium   | FR-103:48 `SHALL pin the submodule … and SHALL report a submodule revision change …` is non-singular | FR-103  |
| FND-024 | medium   | FR-103:53 `SHALL record one disposition per … module and SHALL leave no such module unclassified` is non-singular; the second clause restates the first as a completeness assertion and should be a separate criterion | FR-103  |
| FND-025 | medium   | FR-103:63-65 conjoins `SHALL publish no corpus tooling package to public npmjs or crates.io` with `SHALL publish any retained Python package to the internal PyPI` (engine: `unclassifiable` + `missing-subject` at FR-103:64) | FR-103  |
| FND-026 | medium   | NFR-027:20-24 Statement chains four `SHALL` obligations (forbid unsafe, workspace lints, one thiserror type per crate boundary, newtype identifiers) plus a fifth on the gates; engine reports `non-singular` + `unclassifiable` + `missing-subject` at NFR-027:22 — split into one NFR statement per obligation or an enumerated `The X SHALL:` list | NFR-027 |
| FND-027 | medium   | NFR-024:19-24 Statement carries three obligations (`SHALL carry that path as an open matrix row …`, `SHALL declare every non-retention allowance …`, `SHALL report a retained path as retained-with-successor …`) but is engine-clean because each `SHALL` lands on its own wrapped line — genuine `non-singular` the engine cannot see | NFR-024 |
| FND-028 | medium   | NFR-026:19-22 Statement carries three obligations (declare the toolchain channel, set it to 1.98.1, fail every gate on an older toolchain) and is likewise engine-clean only because of line wrapping | NFR-026 |
| FND-029 | low      | StR-009:28-32 Stakeholder Need carries two lowercase `shall` obligations (implement behind one versioned boundary; source every boundary type from one schema source); acceptable under the StR dialect but it splits cleanly into two needs | StR-009 |
| FND-030 | low      | Wrapped multi-`SHALL` bullets the engine under-counts recur beyond the flagged lines — FR-100:71-74, FR-101:49-50, FR-101:52-53, FR-101:72-73, FR-101:75-76 each carry two obligations on separate physical lines and raise no warning; treat the engine count as a floor, not a census | FR-100, FR-101 |
| FND-031 | medium   | FR-100:65-70 and FR-103:49-52 both bury an unwanted-behaviour clause (`if engineering-assurance does not supply it, then Quoin SHALL file a gap ticket`) inside a ubiquitous statement; lift each into its own `If … then …` requirement so the refusal path gets its own acceptance criterion | FR-100, FR-103 |
| FND-032 | low      | FR-101:62-64 `Removal of a retained path SHALL NOT be recorded as complete while …` names an event, not a system or actor, as its subject; the `While`-shaped condition is correct but the obligation should be restated as `While a criterion … is backed only by …, Quoin SHALL NOT record the removal as complete` | FR-101  |
