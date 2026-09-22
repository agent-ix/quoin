---
id: MP-229
title: Jev EARS lens gate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: jev.ears_gate_verdict
definition_version: jev.ears-gate-v1
relationships: []
---

# Jev EARS lens gate

## Decision Use

Decide GO or NO-GO for wiring the EARS semantic lens (PLAT-838) into
`skills/spec-ears-analysis`, per AP-202's decision boundary. This plan states
the bars before the first live call it judges; it does not itself run
anything or emit findings.

## Population

Every observation reported under `lens: ears` for MP-222 (constant-predictor
margin), MP-223 (defect recall) and MP-224 (no-defect recall) over the M2
labelled fixture corpus, plus MP-231 (engine/semantic delta) over the M6 real-
statement corpus and MP-225 (disagreement under repetition) over the M2
corpus at whatever `variant` and `N` were actually run.

The M2 corpus revision a verdict is computed over is part of the verdict.
PLAT-838's first run used a 16-fixture, 5-defect revision; the `v3` run uses
a 59-fixture, 19-defect revision (31 rows verbatim from real agent-ix spec
trees, 28 authored), holding the defect share at 32.2% against the original
31.2% so the constant-predictor baseline in bar 1 does not move merely
because defects were added. Every label in either revision is
**agent-labelled, not human ground truth**, and a verdict that does not say
so is misreported.

## Measure Definition

A composite boolean, `jev.ears-gate-v1`: GO only when every bar in
**Comparison and Enforcement** holds for at least one reported variant. A bar
resting on a `not_computed` upstream observation is itself `not_computed`,
never treated as satisfied.

## Collection Procedure

No collection of its own. Reads the already-collected MP-222/223/224/225/231
observations for `lens: ears` and states the verdict; retains which variant
and corpus revision the verdict was computed against.

## Environment and Sampling

Every upstream metric it reads must share the same corpus revision, question
set and `quire` engine version within one variant; a verdict computed across
mismatched revisions is refused, not reported.

## Interpretation

A GO here is advisory evidence for wiring the lens into the skill as an
**advisory** finding source. Per AP-202 and PLAT-838's own instruction, it is
never itself permission to turn `quire validate --strict` into a hard gate on
EARS findings -- that is a separate, later, user-gated decision, and the M7
bounds below are what would justify raising it, not this section.

## Comparison and Enforcement

**Ship-as-advisory bars (all four must hold; pre-registered here before the
first live call this plan judges):**

1. **MP-222 margin > 0** percentage points — the lens must beat the best
   constant predictor the M2 corpus admits.
2. **MP-223 defect recall > 0%** — the lens must find at least one genuine
   defect in the M2 corpus's non-clean items, not merely score well by
   never firing.
3. **MP-224 no-defect recall > 0%** — the lens must correctly call at least
   one genuinely clean statement clean. This is the exact failure PLAT-917
   measured on the sibling criterion-strength lens (never once answered
   `sound`, flagged all 15 fixtures); a lens scoring zero here is the
   flag-everything failure PLAT-838's own M2 warns against, however its
   margin or recall look.
4. **MP-231 forward delta is both non-zero and exceeds the same variant's
   MP-225 disagreement-under-repetition rate.** A delta at or below the
   lens's own noise floor is not distinguishable from Jev disagreeing with
   itself on identical input; it is not evidence the lens sees something
   the engine misses. (MP-231's inverse delta carries no bar: a low inverse
   delta is evidence about the engine, not a lens defect, and is reported
   for its own sake per MP-231's Interpretation.)

**One classification rule per verdict.** Every bar in a single verdict reads
the same classification rule. A verdict reached under the `v3` noul-derived
rule takes its MP-231 number from `jev.ears-delta-v3`, never from
`jev.ears-delta-v1`; the other version may be reported beside it as a
comparison and carries no bar. This is a statement of what "one variant"
already meant in **Environment and Sampling**, written out because the first
`v3` run did mix the two.

Any bar unmet, or resting on a `not_computed` value, is NO-GO. A lens whose
gate plan is missing or backdated is NO-GO by refusal per AP-202's
Exceptions, independent of any number.

**M7 bounds — later promotion to a hard `quire validate --strict` gate only,
not part of the ship-as-advisory decision above, stated in advance per
PLAT-838's M7:**

- False-positive rate on MP-224's population (`1 - no_defect_recall`) below
  20%.
- MP-226 expected calibration error below 0.15.
- MP-225 disagreement rate below 20%, measured at N ≥ 20 (PLAT-838's stated
  floor for that promotion decision; the ship-as-advisory bar above accepts
  whatever smaller N the variant actually ran, stated honestly as such).

These three numbers are chosen judgment calls, not derived quantities, stated
before any of them is measured so the promotion decision cannot be fitted to
a result after the fact.
