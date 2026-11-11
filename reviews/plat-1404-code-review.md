---
id: SR-20261111201427
title: "Code review of quoin PR 817 (PLAT-1404 paired-run plans in verify and report)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@d9a17f68; rust/crates/quoin-measurement/src/{paired.rs,verify/mod.rs,verify/paired.rs,verify/reason.rs,report/build.rs,report/paired_render.rs,report/render.rs,report/wire.rs,lib.rs}, rust/crates/quoin-measurement/tests/tc_3500_paired_verify.rs, rust/crates/quoin-cli/tests/tc_3513_paired_verify_command.rs"
review_set: subset
---

# Code review of quoin PR 817

## Summary

Ticket: PLAT-1404. Code review with the rust-review lane (repo `rust-style` loaded). The PR
head is d9a17f68 on base ee10e57d. Builds ran through `locked-build.sh` with `-j 4` and
debug=0. `cargo clippy -p quoin-measurement -p quoin-cli --all-targets --all-features -D
warnings` passed. `cargo test -p quoin-measurement -p quoin-cli` passed in full, including
TC-3500..3511, TC-3513, the TC-2500 series, the TC-961 verdict wire and reason census, the
module-size and source-convention tests, and the report golden tests.

A throwaway probe test, not pushed, reproduced these cases:

- **Cookiecutter case.** Plan MP-001 with `relation: equal` and `minimum_pairs: 1`, one
  slice per file, and values that are 48-bit hash prefixes. One collection gives
  `inconclusive [paired_run_unpaired]`. Two equal emissions give `accept []`. One changed
  file gives `reject [paired_run_differs]`, with a decision whose estimate is 1. A dropped
  file gives `reject [paired_run_differs]`.
- **Definition bump.** The report and verify disagree (FND-001).
- **Estimator but no rule.** A paired plan with an estimator and no `decision_rule` gives
  `no_decision_rule` (FND-002).

## Verdict

**Mergeable after two medium fixes (FND-001, FND-002). No high findings.**

- **Honest mapping holds.** A count of 0 is reached only through `reduce`'s `Observed`
  arm. `comparisons < minimum_pairs` and any uncomputed slice are `Inconclusive` before any
  count is taken, so zero differing pairs with `minimum_pairs` unmet cannot accept. `accept`
  is `settle` with no findings.
- **Edge cases.** A one-sided slice counts as differing. A slice with no value is
  inconclusive. A changed build profile is incomparable through FR-044's comparison, and
  `cross_profile_comparison` is honoured there.
- **Duplicates and non-finite values.** The same collection twice accepts, but only
  through the library API. The CLI store keys collections by `collectionId`, so it cannot
  produce that pair. NaN and infinity cannot cross the JSON intake.
- **Exit codes and output for other plans.** Exit codes for non-paired paths are unchanged.
  `settle` and the exit mapping are untouched, and TC-3503 holds `no_decision_rule` for a
  non-paired plan. The non-paired report output is identical by construction:
  `skip_serializing_if = "Vec::is_empty"`, and an empty table emits no lines. The existing
  goldens pass.
- **engineering-assurance consumer.** Read at origin/main, `src/workflow_invariants.rs`
  `MeasurementVerdict` has `deny_unknown_fields` at the top level only. `reasons` is
  `Option<Vec<String>>`, so any code string is accepted. `decisions` is
  `Option<Vec<Value>>`, opaque and never read, so empty `dimensions` and `baseline: 0` are
  accepted. A non-`accept` verdict maps to `not_accepted`. The JS provider
  (`invariants.js`) copies `reasons` verbatim.
- **No other quoin consumer parses verdict reason codes.** The campaign verdict reads only
  the verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `quoin report` picks a paired plan's collections with `observes(candidate, plan.metric)`, which checks the metric only. `verify` uses `observations_of`, which also checks plan id and `definition_version`. FR-132 Behavior says both use the same membership. Reproduced: two agreeing v2 collections followed by a v1 collection of the same metric. `verify` returns accept, while the report shows `incomparable` with the reason "definition moved v2 -> v1". A second plan sharing the metric would also pair the wrong plan's collections. Filter `measuring` by plan id and `definition_version`, and add a test | rust/crates/quoin-measurement/src/report/build.rs:174-178, rust/crates/quoin-measurement/src/report/build.rs:237-242 |
| FND-002 | medium | A plan that declares `paired_run` and an `estimator` but no `decision_rule` matches `(None, _)` and still gets `no_decision_rule`. Reproduced: inconclusive `[NoDecisionRule]`. The FR-108 table row this PR amends says `no_decision_rule` applies only when the plan "declares no `paired_run`". Either the guard should be `(None, _) if pair.is_some()`, or the FR-108 row should say "states neither `decision_rule` nor `estimator`" | rust/crates/quoin-measurement/src/verify/mod.rs:152-153, spec/functional/FR-108-measurement-verdict-checker.md:137 |
| FND-003 | low | When a plan declares both a rule and `paired_run`, `decisions` mixes per-slice rule decisions with the paired count, which also has empty `dimensions`. A reader cannot tell the count apart from a real slice decision of a dimensionless observation. engineering-assurance ignores `decisions`, so nothing breaks today. The entry is only identifiable by convention | rust/crates/quoin-measurement/src/verify/paired.rs:45-54 |
