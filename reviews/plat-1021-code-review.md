---
id: SR-20261111193746
title: "Code review of quoin PR 811 (graded run measurement record)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@07cd5722; rust/crates/quoin-jev/src/{graded_run,lib}.rs, rust/crates/quoin-jev/tests/{live_criterion_strength,tc_1021_graded_run}.rs, rust/crates/quoin-jev/tests/support/{grading,mod}.rs, rust/crates/quoin-cli/tests/tc_1021_graded_run_verify.rs, rust/crates/quoin-cli/Cargo.toml, rust/Cargo.lock, spec/functional/FR-134-graded-run-measurement-record.md, spec/{matrix,spec}.md, spec/functional/index.md"
review_set: subset
---

# Code review of quoin PR 811

## Summary

Ticket: PLAT-1021. Scoped to `git diff origin/main...HEAD` at 07cd5722. The Rust lane
(`rust-review`, with the repo's `rust-style`) is folded in. Builds ran through
locked-build.sh with debug=0 and jobs=4. `cargo test -p quoin-jev` passes in full on the
default path. `tc_1021_001` and `tc_1021_101` pass, and so does `quoin-core
tc_1045_spec_integrity`. `cargo clippy -p quoin-jev -p quoin-cli --all-targets
--all-features --locked -D warnings` is clean, and so are `cargo fmt --all --check` and
`cargo deny check`. A throwaway probe (reverted, never pushed) replaced the e2e plan's
metric with MP-228's `jev.criterion_strength.gate`. `quoin measurement record` then
refused the input with "metric `jev.criterion-strength.agreement-rate` has no
MeasurementPlan", which confirms FND-003.

These parts are clean. Item rows equal the items one for one, with duplicates refused by
`item_observations`, so rows cover `population.examined` exactly. The baseline arithmetic
in the e2e test is right: `weakness_kind` best constant `sound` = 6 primary + 3 contested
= 9/11, and `coverage` best constant `2` = 3/4, so the baseline is 12/15 = 0.8. Accept and
reject are read from the `verify` output (`verdict`, `reasons`, `decisions[0].baseline`),
not hardcoded. Families are one per row kind (`is_coverage`) and match the fixture plan's
`answer_families`, so PLAT-1020's set-equality check is exercised. The quoin-jev
dev-dependency adds one Cargo.lock edge and nothing else. It does not reach quoin-cli's
default or release build. It does compile the typesafe SDK into quoin-cli's test build,
but the workspace test build already compiles it through quoin-jev. `cargo deny` passes.

## Verdict

**NOT MERGEABLE (three high).** This ticket already has a committed spec,
FR-108-AC-15..AC-23 with TC-2013..TC-2022 (quoin#779). The matrix FR-108 row says
"PLAT-1021 specifies, and has not yet built, the graded-corpus collection producer". The
PR ignores that spec. It mints a parallel FR-134 whose contract contradicts it in several
places. Its live hook states a false build profile and a metric that no plan governs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR adds FR-134 (TC-2800/2801) for work that committed FR-108-AC-15..AC-23 (TC-2013..TC-2022) already specify for this same ticket, and leaves those ACs untagged and marked as unbuilt in the matrix. FR-134 is a duplicate spec with a weaker, contradicting contract (FND-002, FND-004..FND-007). Fix: implement against FR-108-AC-15..23, tag TC-2013..2022, and delete FR-134 and its index, spec.md and matrix rows. If the contract really needs to change, amend FR-108 instead | spec/functional/FR-134-graded-run-measurement-record.md, spec/functional/FR-108-measurement-verdict-checker.md:190-198, spec/matrix.md:127,406-415 |
| FND-002 | high | `record_input` hardcodes `verificationStack.buildProfile: "release"`. The live harness runs under `cargo test`, which is a debug build, so every recorded run falsely attests a release build. This violates FR-108-AC-22: state no `buildProfile`, take the stack from the caller, and never let a debug build state `release` | rust/crates/quoin-jev/src/graded_run.rs:91-96 |
| FND-003 | high | The live hook hardcodes `metric: "jev.criterion-strength.agreement-rate"`. No MeasurementPlan in the repo governs that metric, including MP-228 (`jev.criterion_strength.gate`), which FR-108-AC-21 names as the intended consumer. `JEV_PLAN_ID` selects the plan but not the metric, so `quoin measurement record` refuses every real run's input. The ticket's acceptance ("a real `quoin measurement record` of a Jev grading run stores the item rows") cannot be met. Take the metric from the governing plan, either by reading it from `JEV_PLAN_PATH` or from an env var | rust/crates/quoin-jev/tests/live_criterion_strength.rs:768-804 |
| FND-004 | medium | `record_run` runs after the unanswered and unrecognized assertions, so a pass with any unanswered or unrecognized row leaves no stored evidence. FR-108-AC-21 requires the write to come before every assertion, including those checks. MP-222 also keeps such items in the denominator as disagreements | rust/crates/quoin-jev/tests/live_criterion_strength.rs:831-862 |
| FND-005 | medium | `matched` is recomputed by string equality on `actual_class` and ignores the row's `Verdict`. `grade_weakness` marks a row `Unrecognized` whenever `verdict.unrecognized` is non-empty, but takes `actual` from `findings[0]`. Such a row whose first finding equals `expected` is therefore counted as matched in the record and as not agreeing in the gate. This is unreachable today only because of FND-004. Once the write moves before the asserts, as AC-21 requires, it violates FR-108-AC-20. `constant_predictor_items` should map non-agreeing verdicts to a non-matching class | rust/crates/quoin-jev/tests/support/grading.rs:376-392, rust/crates/quoin-jev/src/graded_run.rs:62-65 |
| FND-006 | medium | Item rows follow input order, not `item_id` order, so permuting the items changes the document. This violates FR-108-AC-17 and AC-19, which require rows in `item_id` order and a permutation-invariant result | rust/crates/quoin-jev/src/graded_run.rs:61,79-81 |
| FND-007 | medium | The envelope diverges from FR-108-AC-15, AC-16 and AC-18 in four ways. `rawEvidence` is `null` where AC-15 requires an empty object. `toolIdentity`, `environment` and `verificationStack.capabilities` are invented rather than taken from the caller. The aggregate carries no `repetitions`. An empty `metric` is not refused: the aggregate is emitted with `metric: ""` | rust/crates/quoin-jev/src/graded_run.rs:54-101 |
| FND-008 | medium | No test exercises the contested-only branch of `matched`, where `actual` equals a contested alternate but not the primary reading. Every agreeing item in both tests has `actual == expected`. A mutation that drops `item.contested.contains(&item.actual)` survives both tc_1021 tests, even though FR-134 Behavior and FR-108-AC-16 both state that branch | rust/crates/quoin-jev/tests/tc_1021_graded_run.rs:29-37, rust/crates/quoin-cli/tests/tc_1021_graded_run_verify.rs:89-97 |
| FND-009 | low | The agreement predicate is duplicated: `record_input` re-implements `item_observations`' `agreed` test instead of counting rows whose `value` is 1.0. Also, the `Item.actual` doc still says "Not read by the constant-predictor formula ... for traceability", but `record_input` now reads it to compute `matched` | rust/crates/quoin-jev/src/graded_run.rs:62-65, rust/crates/quoin-jev/src/constant_predictor.rs:79-83 |

## Examined scope

```yaml
clean: false
scope:
  - {id: record_input, path: rust/crates/quoin-jev/src/graded_run.rs, role: examined, excerpt: "let rows = item_observations(run.plan_id, run.definition_version, run.metric, items)?; let matched = items.iter().filter(|item| item.actual == item.expected || item.contested.contains(&item.actual)).count();"}
  - {id: record_run, path: rust/crates/quoin-jev/tests/live_criterion_strength.rs, role: examined, excerpt: "metric: \"jev.criterion-strength.agreement-rate\", ... let input = quoin_jev::record_input(&run, &constant_predictor_items(graded))"}
  - {id: constant_predictor_items, path: rust/crates/quoin-jev/tests/support/grading.rs, role: examined, excerpt: "family: if is_coverage(row) { \"coverage\" } else { \"weakness_kind\" } ... actual: row.actual_class.clone()"}
  - {id: tc_1021_101, path: rust/crates/quoin-cli/tests/tc_1021_graded_run_verify.rs, role: examined, excerpt: "15/15 agreement clears the 0.8 baseline plus the 0.05 margin ... 12/15 equals the baseline"}
  - {id: tc_1021_001, path: rust/crates/quoin-jev/tests/tc_1021_graded_run.rs, role: examined, excerpt: "assert_eq!(aggregate[\"population\"][\"examined\"], 3); assert_eq!(aggregate[\"population\"][\"matched\"], 2);"}
  - {id: quoin-cli dev-dependency, path: rust/crates/quoin-cli/Cargo.toml, role: examined, excerpt: "quoin-jev = { workspace = true }"}
  - {id: FR-108-AC-15..23, path: spec/functional/FR-108-measurement-verdict-checker.md, role: context_only, excerpt: "quoin_jev::graded_corpus_collection takes a GradedCorpus ... returns a candidate MeasurementCollection"}
```
