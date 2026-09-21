---
id: MP-228
title: Jev criterion-strength lens gate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: jev.criterion_strength.gate
definition_version: jev.criterion-strength-gate-v1
relationships: []
---

# Jev criterion-strength lens gate

## Decision Use

Decide GO or NO-GO for wiring the criterion-strength lens (PLAT-837) into the
`spec-criterion-strength-analysis` skill, under AP-202.

## Population

The fifteen human-labelled fixtures in
`skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fixtures.json`:
eleven `weakness_kind` criteria and four `adverse_case_coverage` FRs. Nine carry
a second reader's contested reading. Ground truth is human-labelled.

## Measure Definition

Three bars over one graded pass, each read from a shared metric plan: margin
over the constant predictor (MP-222), defect recall (MP-223) and no-defect
recall (MP-224), definition `jev.criterion-strength-gate-v1`. Disagreement
(MP-225), calibration (MP-226) and cost and latency (MP-227) are reported
beside the gate and do not decide it.

## Collection Procedure

Run `tests/live_criterion_strength.rs` in `rust/crates/quoin-jev` with
`--features live-api`, selecting the variant with `JEV_VARIANT`. The gate is
`the_lens_beats_the_do_nothing_baseline`; the repeated runs are
`repeated_runs_report_a_disagreement_rate` with `JEV_RUNS=5`. Retain the graded
table, token counts, classifier and timestamp for each pass.

## Environment and Sampling

Three pre-registered variants: `v0` sends the corpus fields only, `v1` adds the
full FR context from `criterion-strength-fr-context.json`, and `v2` adds a
neutral `weakness_kind` question. One gate pass per variant, then five repeated
passes for any variant under consideration. A transport failure aborts the pass
and is never scored.

## Interpretation

Fifteen fixtures is a small corpus. A GO is evidence that the lens beats a
lookup table and clears sound criteria on this corpus. It says nothing about
recall on defect kinds the corpus does not hold.

## Comparison and Enforcement

The bars below were first committed in `55c467d9a57d21be3c15ff0bbf1be0db1e4b429c`
(agent-ix/quoin, branch `plat-jev-live-tests`), in the module doc of
`rust/crates/quoin-jev/tests/live_criterion_strength.rs`, before any round-two
call to the service. They are reproduced here word for word.

**Bars.** A variant passes when all three hold on one graded pass:

1. agreement > the best constant predictor, one constant per family
   ([`support::trivial_baseline`]);
2. defect recall > 0 over the criteria no reader called `sound`;
3. **`sound` returned on at least 3 of the 5 criteria whose primary
   reading is `sound`** -- a false-positive rate on sound criteria below
   50%. Below that, a flag from this lens on a criterion is more likely
   noise than signal, and PLAT-837's M2 names false positives as the
   headline cost because each one costs a human read.

**GO** requires one variant to pass all three bars on the gate run *and*
in at least 3 of 5 repeated runs (`JEV_RUNS=5`). Fifteen fixtures and one
run can pass by luck; a variant that fails most repeats is not a result.
