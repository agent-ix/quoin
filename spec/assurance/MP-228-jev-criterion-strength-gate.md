---
id: MP-228
title: Jev criterion-strength lens gate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: jev.criterion_strength.gate
definition_version: jev.criterion-strength-gate-v1
ground_truth_kind: agent-labelled
relationships: []
---

# Jev criterion-strength lens gate

## Decision Use

Decide GO or NO-GO for wiring the criterion-strength lens (PLAT-837) into the
`spec-criterion-strength-analysis` skill, under AP-202.

## Population

The fifteen fixtures in
`skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fixtures.json`:
eleven `weakness_kind` criteria and four `adverse_case_coverage` FRs. Nine carry
a second reader's contested reading. Ground truth is agent-labelled, not
human: the corpus's own `governing_ruling_on_disagreement` field and
`SKILL.md` claim "a human label" and "a second, independent reader," but
`gh pr view agent-ix/quoin#570 --json reviews,comments,commits` shows zero
GitHub reviews, zero comments, and both authoring commits by `claude` — the
"second reader" was another Claude Code session, not a person.

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

Two follow-up variants were pre-registered and measured after round two, each
in the module doc of `live_criterion_strength.rs` before its first call: `v3`
derives the label from the five `noul` answers, and `v4` sends described
`choice` criteria. Both are NO-GO. `v5` is below.

### `v5` (PLAT-979), pre-registered before any `v5` call

PLAT-979 asks for one wording or decomposition retry before the NO-GO is
treated as final. Its method: classify every disagreement as wording, missing
sub-question, bad option list, threshold or bad label, in that order, and fix
the dominant class first.

**Step 1: a diagnostic re-run of `v2`, reported only.** No per-row table from
any earlier pass was kept, so the method has no input. `v2` is re-run once,
unchanged, to classify its disagreements. `v2` is the variant to start from:
it is the only one of five that ever cleared `sound`, and it scored highest.
This pass scores no new variant, and nothing about `v5` is chosen before it.

**Step 2: `v5`,** designed from step 1's dominant class, is pre-registered in
its own commit before its first call. The bars are unchanged.

**Stated in advance.** Whoever writes `v5` has read the fixtures and
their rationales. `v5` is therefore fitted to the corpus in a way `v3` and
`v4` were not. A GO from `v5` is weak evidence and would need a held-out corpus
before any wiring. A NO-GO from `v5` is the stronger result: the lens fails
even when tuned against its own answer key.

**What bar 1 means on this corpus.** The constant predictor gets 12 of 15
rows right. It misses only `CS-FIX-003`, `CS-FIX-005` and `CS-FIX-011`, so
beating it means getting 13 or more right, with at most two errors. `sound` is
an accepted reading on 9 of the 11 criteria. On those rows a lens can only
match the constant or lose to it.

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
