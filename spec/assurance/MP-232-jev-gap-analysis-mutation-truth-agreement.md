---
id: MP-232
title: Gap-analysis lens agreement against mechanical mutation truth
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.mutation_truth_agreement
definition_version: jev.mutation-truth-v1
relationships: []
---

# Gap-analysis lens agreement against mechanical mutation truth

## Decision Use

Decide whether the gap-analysis lens's `assertion_vacuous` answer is grounded
in the same fact a mutation-testing tool would find, rather than in wording
that merely looks thorough.

## Population

Every requirement<->test<->code triple in the gap-analysis corpus whose
covered symbol was actually stubbed to a default-returning body and whose one
owning test was rerun against that stub. A triple with no stub run — its
return type admits no safe default, so no mutation could be performed — stays
out of the population and is not scored as either class.

## Measure Definition

Agreement between the lens's `assertion_vacuous` answer, thresholded at 0.5,
and the mutation outcome (the test still passed under the stub -> vacuous;
the test caught the stub -> not vacuous), definition `jev.mutation-truth-v1`.
Reported alongside MP-222's constant-predictor margin, MP-223's defect
(vacuous-class) recall and MP-224's no-defect (not-vacuous-class) recall over
the same population — this plan states what the ground truth IS for
gap-analysis, not a fourth way of scoring it.

## Collection Procedure

Stub the covered symbol in a scratch, uncommitted edit of the worktree; rerun
exactly the one owning test; record pass or fail; revert before any commit —
nothing mutated is ever committed. Retain which symbol, what stub, and the
pass/fail outcome per triple. Run the lens's `live-api` test through
`quoin_jev::client::production` over the same triples. Dimension every
observation by `lens` and `variant`.

## Environment and Sampling

The mutation campaign is a small sampled subset, per its own scoping ticket —
not a full mutation-testing run. A triple excluded from the mutated subset is
excluded from this measure's population, never defaulted to either class.

## Interpretation

This is the strongest ground truth in the set: checked against the compiler
and the test runner, not a human labeller. A high agreement here is stronger
evidence than the same number against a human-labelled corpus, because the
ground truth cannot itself be contested the way a labeller's reading can.

## Comparison and Enforcement

This plan assigns no verdict. MP-230 states gap-analysis's bar on this
metric. Compare only like lens, variant, corpus revision and mutation log.
