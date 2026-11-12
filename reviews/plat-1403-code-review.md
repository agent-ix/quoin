---
id: SR-17079
title: "Code review of quoin PR 816 (PLAT-1403 producers state population ids for MP-219, MP-229, MP-230)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@a0b62b2c; rust/crates/quoin-cli/tests/nfr_populations/{mod.rs,span_breadth.rs}, rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs, rust/crates/quoin-jev/tests/{live_ears_semantic.rs,live_gap_semantic.rs,support/gate.rs,support/mod.rs,tc_3302_gate_observations.rs}"
review_set: subset
date: '2026-11-11'
---

# Code review of quoin PR 816

## Summary

Ticket: PLAT-1403. Scoped to `git diff origin/main...HEAD` at a0b62b2c. The base is ee10e57d, which is current origin/main and includes the merged #815. The Rust lane (`rust-review`, with quoin's `rust-style`) is folded in. Builds ran through locked-build.sh with debug=0 and 4 jobs, in a separate review worktree.

- `cargo test -p quoin-cli --test tc_3003_nfr_populations -- --include-ignored`, with `QUIRE_RS_CHECKOUT` set to a quire-rs origin/main worktree (05afc40): 6 passed. TC-3301 printed `MP-219: 32 of 40 labels reproduced; verdict Reject [ApparatusUnrecorded, RuleNotMet]`.
- `cargo test -p quoin-jev --test tc_3302_gate_observations --test tc_2022_graded_items`: 5 passed.
- `cargo clippy -p quoin-jev -p quoin-cli --all-targets --features quoin-jev/live-api -D warnings`: clean.
- The ears and gap gate files keep `#![cfg(feature = "live-api")]`, so the default gate makes no live Jev call and opens no socket. TC-3302 is offline.
- `bench/span-breadth-v1-labels.json` is untouched. It does not appear in the diff.

## Verdict

**NOT MERGEABLE until FND-001 is fixed.** The fix is small.

The design is honest:

- **Jev gates (MP-229, MP-230).** Each bar's `met` is the same predicate as the gate assertion that follows it: margin > 0, recall > 0, and forward delta > 0 and above MP-225. A `not_computed` value maps to `met: false`, which matches the plan's error model. The observation is recorded before the assertions, so a NO-GO is stored with matched < examined, and value < 1.0 rejects under `ge 1.0`. The ids are literals at the call site, next to the code that computes each bar. Intake therefore guards only against a missing or invented id. A harness that computed nothing could not produce `met: true` except through a hard-coded literal.
- **MP-219 replay.** It refuses when a checkout is missing. It examines every label and matches only on exact spans or the reviewed refusal signal.
- **Ordering and side effects.** Ordering is deterministic (`BTreeMap`/`BTreeSet`). No work is spawned. TC-3300 uses the fixture repository's own module, so results do not depend on ambient modules.
- **Shared harness.** Renaming `nfr-`/`nfr.probe` to `population-`/`population.probe` in the shared harness has no consumer anywhere in the repository.

The comparison of the label statement is wrong (FND-001):

- The replay compares the classifier's statement against the label's `normalizedStatement`, which uses a different normalization from the replay's.
- Of the eight unmatched labels, seven are genuine drift that I re-measured: FR-029-AC-12 is withdrawn, FR-029-AC-14 is gone, and FR-030-AC-3/-8/-13, FR-031-AC-23 and FR-048-AC-2 are reworded.
- The eighth, `quire-rs-FR-039-AC-5`, is a false negative produced by the producer itself.

The `ApparatusUnrecorded` in the printed verdict comes from the harness: it does not resolve apparatus at intake. That reason is inconclusive-only and never rejects. It does not misreport anything here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The MP-219 replay compares `criterion.statement` against the label's `normalizedStatement` after lowercasing and collapsing whitespace. The labeller's normalization also folded Unicode: `…` became `...`. So `quire-rs-FR-039-AC-5` is reported as not matched, although its statement in quire-rs is byte-identical to the label's `statement` and `quire properties` returns the reviewed `span:refused-weak-boundary` refusal with no spans. It is the only label whose `normalizedStatement` differs from lowercase+whitespace of its `statement`. The true replay is 33/40, not 32/40: still Reject, but the measured value is wrong. Fix: compare against the label's `statement` with whitespace collapsed, or apply the same normalization to both sides. Add a TC-3300 fixture label with such a character | rust/crates/quoin-cli/tests/nfr_populations/span_breadth.rs:75-88 |
| FND-002 | low | TC-3301 asserts `probed.examined.len() == 40`, which pins a count. The following `verdict("MP-219")` check, which finds no population reason, already proves the examined ids equal the declared set. The literal count adds nothing and breaks on any label-set change. Delete it | rust/crates/quoin-cli/tests/tc_3003_nfr_populations.rs:202 |
| FND-003 | low | The bar predicates are written twice: once in the `record_ears_gate`/`record_gap_gate` arguments and once in the gate `assert!`s that follow. Both copies agree today. A later edit to one side would record a bar as met while the assertion fails, or the reverse. Compute each bar's bool once and pass the same value to the observation and to the assertion | rust/crates/quoin-jev/tests/live_ears_semantic.rs:332-361, rust/crates/quoin-jev/tests/live_ears_semantic.rs:960, rust/crates/quoin-jev/tests/live_ears_semantic.rs:1227, rust/crates/quoin-jev/tests/live_gap_semantic.rs:229-256 |
| FND-004 | low | An unmatched label carries no reason, in either the collection or the TC-3301 output. A missing row, a drifted statement and a different classifier outcome all look the same, which is how FND-001 went unnoticed and was reported as drift. Print or record the reason for each unmatched label (row-missing, statement-drift or outcome-differs) | rust/crates/quoin-cli/tests/nfr_populations/span_breadth.rs:128-139 |
