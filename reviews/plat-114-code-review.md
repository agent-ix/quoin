---
id: SR-20261111190501
title: "Code review of quoin PR 808 (paired-run metrics, FR-132)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@e29f1c25; rust/Cargo.toml, rust/Cargo.lock, rust/crates/quoin-measurement/src/paired.rs, rust/crates/quoin-measurement/src/plans.rs, rust/crates/quoin-measurement/src/plans/design.rs, rust/crates/quoin-measurement/src/types/plan.rs, rust/crates/quoin-measurement/src/report/verdict.rs, rust/crates/quoin-measurement/tests/tc_2500_paired_runs.rs, struct-literal edits in quoin-jev, quoin-measurement-graph and quoin-measurement tests"
review_set: subset
---

# Code review of quoin PR 808

## Summary

Ticket: PLAT-114. Scoped to `git diff origin/main...HEAD` at e29f1c25. The Rust lane
(`rust-review`, with the repo `rust-style`) is folded in. Evidence gathered through
`locked-build.sh`: `cargo metadata --locked` succeeds; the lock differs from current
`origin/main` by exactly the two `source =` lines of the engineering-assurance packages;
`cargo clippy --locked --workspace --all-targets --all-features -- -D warnings` exits 0;
`cargo test --locked -p quoin-measurement --test tc_2500_paired_runs` passes 7 of 7.
Throwaway probe tests (not committed) confirmed the findings below.

## Verdict

**NOT MERGEABLE AS IS (one high, two medium, one low).** The structure is right: one
comparison engine, `NonZeroU32` for `minimum_pairs`, no zero-comparisons-means-pass path,
a blocked slice makes the pair incomparable with its reason named, the within_delta band
is inclusive and the test pins that boundary, NaN/inf/negative delta are refused at load,
duplicate slice keys are refused by collection validation before comparison. The lock edit
is consistent and carries no unrelated churn. The defect is that a slice present in only
one run is dropped silently, so a run that loses slices reads as zero variance whenever the
remaining shared slices reach `minimum_pairs`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `reduce` keeps only `Comparable` rows and ignores `NotComputed` rows, so a slice present in one run only is neither a violation nor a block. Probe: first run roots r1,r2,r3, second run r1,r2 with equal values, `minimum_pairs: 2` gives `Observed { differing: 0 }`. FR-132-AC-4 says slices present in one run only never read as zero variance, and for the in-scope metrics (catalog ordering per root, repeat-reconcile) a root that vanishes is exactly the disagreement to catch. Count a one-sided slice as a violation, or make the pair inconclusive/incomparable naming it, and test the case with enough shared slices | rust/crates/quoin-measurement/src/paired.rs:97-107 |
| FND-002 | medium | `paired_run_from` refuses a block without `minimum_pairs`, but EA FR-034 at 4e8df92 makes `minimum_pairs` optional with the default left to the consumer. A plan EA's schema accepts fails `load_measurement_plans`, which fails the whole plan set. Either state a quoin default (for example 1, still inconclusive on zero comparisons) or keep the refusal and say in FR-132 that quoin deliberately requires it | rust/crates/quoin-measurement/src/plans/design.rs:318-325 |
| FND-003 | medium | TC-2502 cannot fail when the objective direction or the run order is ignored: lower, higher and higher-reversed all assert `differing: 1`, and a `violates` that always ranks as `lower` passes all three. FR-132-AC-3 says the counts differ when direction or order reverses. Use fixtures where the counts differ, for example first (5,5), second (4,4): lower 0, higher 2, higher reversed 0 | rust/crates/quoin-measurement/tests/tc_2500_paired_runs.rs:572-597 |
| FND-004 | low | The reader accepts `delta: 0` and unknown members inside `paired_run`; EA's schema refuses both (`exclusiveMinimum: 0`, `additionalProperties: false`). The read is not the schema's definition. Refuse both, and change FR-132's "non-negative" to "greater than 0" | rust/crates/quoin-measurement/src/plans/design.rs:326-338 |
