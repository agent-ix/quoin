---
id: SR-20261111190502
title: "Gap analysis of quoin PR 808 (FR-132 paired-run metrics)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@e29f1c25; spec/functional/FR-132-paired-run-metrics.md, spec/matrix.md, rust/crates/quoin-measurement/src/paired.rs, rust/crates/quoin-measurement/tests/tc_2500_paired_runs.rs"
review_set: subset
---

# Gap analysis of quoin PR 808

## Summary

Ticket: PLAT-114. Planless; plan completion: not assessed. `quoin matrix` (quoin 0.30.0)
lists FR-132-AC-1..AC-7 each `tagged` by TC-2500..2506 and reports no
criterion-tagged-outside-row diagnostic. FR-132 and TC-2500..2506 collide with nothing on
`origin/main` or in any open PR diff (including #806 and #807). No NFR row is bound in
`spec/evidence/bindings.json` and neither FR-132 nor the PR body claims NFR-001-M-2 or
NFR-002-M-1 is measured; FR-132 describes the mechanism only and `compare_paired_runs` has
no production caller, which the PR states as the follow-up.

## Verdict

**PASS WITH FINDINGS (two medium, one low).** Every AC has a tagged test; AC-1, AC-2, AC-6
and AC-7 tests can fail on the behaviour they claim. AC-3 and AC-4 tests are too weak to
back their whole criterion, and AC-5 is backed for two of its four named reasons.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-132-AC-4 claims slices present in one run only never read as zero variance; TC-2503 tests that only where the shared slices fall below `minimum_pairs`. With enough shared slices the code reads zero variance (code-review FND-001), and no test covers it | rust/crates/quoin-measurement/tests/tc_2500_paired_runs.rs:600-615 |
| FND-002 | medium | FR-132-AC-3's "count differently when the direction or the order of the runs reverses" is not backed: TC-2502 asserts the same count in all three cases (code-review FND-003) | rust/crates/quoin-measurement/tests/tc_2500_paired_runs.rs:572-597 |
| FND-003 | low | FR-132-AC-5 names apparatus changed and incomplete population; TC-2504 exercises only profile and definition changes. The reduction is shared, so add one incomplete-population case to cover the remaining path through `compare_paired_runs` | rust/crates/quoin-measurement/tests/tc_2500_paired_runs.rs:617-654 |
