---
id: MP-229
title: Jev EARS lens gate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: jev.ears_gate_verdict
definition_version: jev.ears-gate-v1
ground_truth_kind: agent-labelled
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

## Measured outcome -- GO (ship as advisory), 2026-09-22

MEASURED by `the_ears_lens_gate_mp_229_v3` at `variant: v3`, N=3 passes over
the 59-fixture M2 revision plus one pass over the 45-statement M6 corpus, 222
requests. Full record:
`spec/evidence/measurements/plat838-ears-v3-20260922T0420Z.json`.

| bar | metric | value | met |
| --- | --- | --- | --- |
| 1 | MP-222 margin over the best constant predictor (`clean`, 69.5%) | **+13.6 pp** (agreement 83.1%) | yes |
| 2 | MP-223 defect recall | **61.1%** (11/18) | yes |
| 3 | MP-224 no-defect recall | **90.0%** (36/40) | yes |
| 4 | MP-231 `jev.ears-delta-v3` forward delta vs MP-225 0.0% | **30.0%** (6/20) | yes |

Every bar reads the same `v3` classification rule, which is what the first
`v3` run did not do. The shipped six-way-choice rule, graded from the
identical responses, scores **-13.6 pp** on bar 1 -- it fails, and deleting
the six-way term is what carries this verdict.

**What the verdict is, exactly.** GO to wire the lens into
`skills/spec-ears-analysis` as an **advisory** finding source, per
**Interpretation** above. It is not permission to make EARS findings a hard
`quire validate --strict` gate; the M7 bounds below govern that, and two of
the three are not met (ECE 0.3551 against a 0.15 bound, and MP-225's 0.0% is
measured at N=3 against a stated N>=20 floor). The false-positive bound is
met at 10.0%.

**Three facts a reader of this GO must carry with it.**

1. **Defect recall fell from 100% to 61.1% the moment the corpus could
   measure the v3 rule's stated blind spot.** On the 13 defects the rule can
   reach it scores 76.9%; on the five it structurally cannot
   (`while_is_really_when`, `if_is_really_when`, `where_is_really_if`,
   `where_is_really_while`, `missing_trigger`) it scores 20.0%. The 100% the
   16-fixture run reported was a fact about that corpus, not about the rule.
   An advisory lens that cannot see a `While` that should be a `When` is
   still worth shipping; one advertised as covering EARS pattern confusion
   generally is not.
2. **MP-231's inverse delta is 88.0% under `v3`, against 12.0% under `v1` on
   the same responses.** This is not evidence that the engine raises 88%
   false positives. The `v3` rule reads measurability and the two
   When-disambiguators only, so it has no question that could corroborate
   `ears:missing-subject`, `ears:non-singular` or `ears:unclassifiable` --
   the codes this pool is flagged under. MP-231 assigns no bar in this
   direction, and this number must not be quoted as an engine defect rate.
3. **The lens is most wrong where it is most confident.** Of 13 rows in the
   0.9-1.0 confidence bucket, 53.8% agreed, against 100% in the 0.4-0.5
   bucket. Confidence must not be surfaced to a user as a reliability
   signal until MP-226 clears its bound.

Every label in the M2 corpus is agent-labelled, not human ground truth. A
restatement of this verdict that drops that sentence is a misreport.
