---
id: MP-230
title: Jev gap-analysis lens gate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: jev.gap_analysis_gate_verdict
definition_version: jev.gap-analysis-gate-v1
ground_truth_kind: mechanical
protected_apparatus:
  - rust/crates/quoin-jev/tests/fixtures/gap-semantic-corpus.json
  - rust/crates/quoin-jev/tests/fixtures/gap-battery-mutants.json
  - rust/crates/quoin-jev/tests/fixtures/gap-battery-mockonly.json
  - rust/crates/quoin-jev/tests/fixtures/gap-battery-mutants-v1-additions.json
negative_controls:
  - kind: apparatus-edit
    description: this gate has no collection of its own (see Collection Procedure) -- its verdict is only as good as the corpus and mutation fixtures MP-222/223/224/232 measured against, so an edit to any of them since measurement invalidates the read the same way it would invalidate those upstream observations
  - kind: suppressed-observation
    description: a bar resting on a not_computed upstream observation is itself not_computed, never treated as satisfied, per Comparison and Enforcement
relationships: []
---

# Jev gap-analysis lens gate

## Decision Use

Decide GO or NO-GO for treating the gap-analysis semantic lens (PLAT-839) as
proven worth integrating into `skills/gap-analysis` step 5, per AP-202's
decision boundary. This plan states the bars before the first live call it
judges; it does not itself run anything, emit findings, or wire the lens into
the skill — that stays a separate, later, user-gated decision even on a GO
verdict here.

## Population

Every observation reported under `lens: gap-analysis` for MP-222
(constant-predictor margin), MP-223 (defect / vacuous-class recall) and
MP-224 (no-defect / not-vacuous-class recall) over the mutation-graded subset
MP-232 defines, plus MP-225 (disagreement under repetition) over the same
subset, at whatever `variant` and `N` were actually run.

## Measure Definition

A composite boolean, `jev.gap-analysis-gate-v1`: GO only when every bar in
**Comparison and Enforcement** holds for at least one reported variant. A bar
resting on a `not_computed` upstream observation is itself `not_computed`,
never treated as satisfied.

## Collection Procedure

No collection of its own. Reads the already-collected MP-222/223/224/225/232
observations for `lens: gap-analysis` and states the verdict; retains which
variant and corpus revision the verdict was computed against.

## Environment and Sampling

Every upstream metric it reads must share the same corpus revision, question
set and mutation log within one variant; a verdict computed across mismatched
revisions is refused, not reported.

## Interpretation

A GO here is advisory evidence only. Per AP-202, it is not itself permission
to remove the opt-in prompt in `gap-analysis` step 5 or to treat a lens
finding as a hard gate — those remain separate, later, user-gated decisions.

## Comparison and Enforcement

**Bars (all three must hold; pre-registered in
`rust/crates/quoin-jev/tests/live_gap_semantic.rs`'s module doc, commit
`d6bb5e4`, before the first live call this plan judges):**

1. **MP-222 margin > 0 percentage points**, over the MP-232 mutation-graded
   subset — the lens must beat the best constant predictor the corpus admits.
2. **MP-223 vacuous-class recall > 0%** — the lens must find at least one
   mechanically-confirmed vacuous test, not merely score well by never
   firing.
3. **MP-224 not-vacuous-class recall > 0%** — the lens must correctly clear
   at least one mechanically-confirmed not-vacuous test. This is the exact
   failure PLAT-917 measured on the sibling criterion-strength lens (never
   once answered `sound`); a lens scoring zero here is the flag-everything
   failure, however its margin or recall look otherwise.

Any bar unmet, or resting on a `not_computed` value, is NO-GO. A lens whose
gate plan is missing or backdated is NO-GO by refusal per AP-202's
Exceptions, independent of any number.

The measured verdict this plan produced, and the observations it was computed
from, are recorded as a measurement collection under
`spec/evidence/measurements/`, not restated here — this plan defines the
rule, not one run's result.
