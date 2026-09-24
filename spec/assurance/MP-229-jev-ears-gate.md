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
protected_apparatus:
  - rust/crates/quoin-jev/tests/fixtures/ears-m2-fixtures.json
  - rust/crates/quoin-jev/tests/fixtures/ears-m6-corpus.json
  - rust/crates/quoin-jev/tests/fixtures/ears-question-set.json
  - rust/crates/quoin-jev/tests/fixtures/ears-question-set-v5.json
negative_controls:
  - kind: apparatus-edit
    description: the M2/M6 fixture corpora and both question-set variants are digested with every collection; an edit to any of them without a new definition version rejects
  - kind: suppressed-observation
    description: a transport failure aborts the pass and is never scored, so a fixture the lens fails to answer cannot be quietly left out of the denominator
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

### Variant log: the PLAT-979 optimization loop

PLAT-979's method is error analysis first. Every disagreement between the lens
and the corpus is read and classified before anything changes. The ticket's
classes, in priority order, are wording, missing sub-question, bad option-list,
threshold and bad label. The dominant class is fixed first, one variant at a
time. Each variant below was written here, with what it is expected to do,
before its first live call. Results go under **Interpretation**.

**The target.** `v3` already clears all four ship-as-advisory bars. The bound
PLAT-979 names for this lens is the failing M7 calibration bound: MP-226 ECE
0.3551 against 0.15. The loop also aims to shrink the eleven remaining `v3`
disagreements.

#### Diagnosis of `v3`, 2026-09-22

The per-row answers came from one fresh `v3` pass over the 59-fixture
revision (`ears_per_row_answers_for_error_analysis`, 59 requests). The pass
agreed on 48 of 59 rows (47 primary, 1 contested on `EARS-FIX-011`). The
recorded 04:20 run agreed on 49. MP-225's 0.0% held within one session and
did not hold across sessions.

| row | miss | answers that decided it | class |
| --- | --- | --- | --- |
| EARS-FIX-017 | defect missed (`when_is_really_if`, "`tsp compile` fails") | `condition_is_unwanted` 0.21 | wording |
| EARS-FIX-019 | defect missed (`when_is_really_if`, unimplemented protocol version) | `condition_is_unwanted` 0.26 | wording |
| EARS-FIX-023 | defect missed (`unmeasurable_response`, "suitable for LLM render agents") | `response_measurable` 0.54 | wording |
| EARS-FIX-033 | clean flagged ("settle with basis `closed-scope`") | `response_measurable` 0.42 | wording |
| EARS-FIX-046 | clean flagged ("SHALL NOT establish complete global success") | `response_measurable` 0.36 | wording |
| EARS-FIX-048 | defect missed (`while_is_really_when`) | `trigger_is_momentary` 0.87, correct but not read on a `While` | rule gap |
| EARS-FIX-050 | defect missed (`if_is_really_when`) | `condition_is_unwanted` 0.08, correct but not read on an `If` | rule gap |
| EARS-FIX-024 | defect missed (`where_is_really_if`) | no question separates a feature condition from a runtime one | missing sub-question |
| EARS-FIX-025 | defect missed (`where_is_really_while`) | same | missing sub-question |
| EARS-FIX-052 | defect missed (`missing_trigger`) | `trigger_is_momentary` 0.92, but clean ubiquitous rows also read 0.47-0.57 on it, so it cannot separate them | missing sub-question |
| EARS-FIX-013 | clean flagged ("When the API receives a webhook payload with an invalid signature") | `condition_is_unwanted` 0.76 | bad label (candidate) |

- **Wording: 5 of 11.** This is the dominant class, and it is also first in
  the ticket's priority order. Both terse `noul` texts are misread in a
  consistent direction:
  - "Does the condition describe an undesired situation?" does not say
    undesired for whom. Failures that the statement handles by design score
    as not undesired.
  - "Is the response measurable?" reads as "is there a number". Checkable
    responses with no number score as unmeasurable.
- **Rule gap: 2 of 11.** These are the nearest fit to the ticket's "missing
  sub-question". The distinction is asked and answered correctly, but `v3`
  consults the two disambiguators only on a `When`.
- **Missing sub-question: 3 of 11.** Nothing asked can tell a `Where` misuse
  or a hidden trigger apart.
- **Bad label: 1 candidate.** `EARS-FIX-013` has the same shape as
  `EARS-FIX-007` ("When the disk quota is exceeded") and `EARS-FIX-017`–`020`,
  which the corpus labels `when_is_really_if` defects. Yet it is labelled
  clean. **Not edited.** Relabelling a row after seeing the lens disagree with
  it fits the answer key to the lens. It is recorded here for a labeller.
- **Threshold: 0 of 11.** No row sits at the 0.5 cut except `EARS-FIX-023`
  (0.54), and its text is a wording case on its face.
- **Bad option-list: 0 of 11.** `v3` does not consult the six-way option
  list at all.

**Calibration is none of the five classes. It is an instrument defect.** Every
grading through `v3` attaches the six-way pick's confidence to each row
(`grade_defect_derived` sets `confidence: verdict.pattern_confidence`). `v3`'s
label never reads the six-way pick. The 0.3551 ECE therefore measures how well
the confidence of one question predicts whether a different question's
derived answer agrees with the corpus. It does not measure the lens's
calibration.

#### `v4` — instrument fix: grade the confidence of the call being graded

- **Change.** The request, the question set and the `v3` label rule are all
  unchanged. MP-226 reads
  `derived_confidence(WhenOnly, …)` (`tests/support/ears.rs`) in place of the
  six-way pick's confidence. The `v3` call is an OR over its consulted
  branches:
  - A `defect` call is as sure as its strongest firing branch (the
    defect-side probability `max`).
  - A `clean` call is as sure as its weakest resisting branch (`1 - max`).

  This mapping follows from the rule's structure. It was chosen before
  computing it on any data, and it was not computed on the diagnostic pass
  above.
- **Expected.** Labels are identical to `v3` by construction, so bars 1-4
  move only by service variance (±1 row, going by the diagnosis above). ECE
  is expected to fall below 0.3551. **No prediction is made about whether it
  clears 0.15.**

#### `v5` — wording: reword the two misread `noul` questions

- **Change.** `v4`, with `ears-question-set-v5.json` in place of the verbatim
  question set. Only these two texts change:
  - `response_measurable` becomes "Does the statement's required response
    name an outcome a tester could check as pass or fail by observation,
    without a subjective judgement about what the words mean?"
  - `condition_is_unwanted` becomes "Does the condition describe a failure,
    error, fault or other situation that is not part of normal operation?"

  Neither text uses a word taken from a fixture.
- **Expected.**
  - `EARS-FIX-017` and `019` flip to defect.
  - `EARS-FIX-033` and `046` flip to clean.
  - `EARS-FIX-023` is uncertain.
  - The named risk is that the broader "checkable" reading lifts
    `response_measurable` above 0.5 on the genuinely unmeasurable rows (`003`,
    `016`, `054`, `055`), which would lose defects.
  - Prediction: net +2 to +4 agreed rows over `v4`, and a bar-1 margin at
    least `v4`'s.
- **If refuted.** If `v5`'s margin is below `v4`'s, the wording hypothesis is
  refuted as worded. `v6` then runs on the verbatim question set, and its test
  is edited to say so before its call.

#### `v6` — read the answers `v3` ignores

- **Change.** `v5`'s wording with `DefectRule::KeywordContradiction`. This is
  `v3`'s rule plus two branches:
  - a `While` whose `trigger_is_momentary` is above 0.5 is really a `When`;
  - an `If` whose `condition_is_unwanted` is below 0.5 is really a `When`.

  No question is added. This is the relationship MP-231's own `v1` text
  already describes ("`condition_is_unwanted`/`trigger_is_momentary`
  contradict the keyword's own implied reading"), which the `v1` code never
  implemented.
- **Expected.**
  - `EARS-FIX-048` and `050` flip to defect.
  - The named risk is that clean `If` rows with `condition_is_unwanted` near
    0.5 become false positives. Under the verbatim wording `037` read 0.43
    and `038` read 0.47. `v5`'s wording is expected to lift them.
  - Prediction: net 0 to +2 rows over `v5`.

**Not attempted in this round.** The three missing-sub-question rows need a
new question: "is this condition a feature or configuration that is either
present or not", and "does this response depend on an event the statement
does not name". That is a request-shape change with its own false-positive
risk on every `Where` and ubiquitous row. It is the next variant if this
round leaves them as the dominant class.

**Overfitting, stated.** Each variant is diagnosed and measured on the same
59 rows, and no held-out split exists. MP-231's forward delta runs over the
M6 real-statement corpus, which no variant was diagnosed on. It is the only
check here against a fix that fits only these fixtures. It is reported for
every variant.

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
requests.

**The record backing this table was withdrawn, 2026-09-23.**
`spec/evidence/measurements/plat838-ears-v3-20260922T0420Z.json` did not
carry `rawEvidence` and recorded `verificationStack.sources.quire-rs` as
`dirty` rather than a clean full-SHA source -- both refused by the ported
intake validator, so the file was never an admissible collection and was
removed rather than backfilled with data not actually measured (agent-ix/quoin#607).
Its bytes remain in git history at `0a257691` (#575). The numbers below are
unbacked by a retained record until a fresh `v3` run is recorded cleanly;
they are not retracted, only unproven by machine-readable evidence pending
that re-run.

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

## `v4`/`v5`/`v6` result, 2026-09-22: all three clear, `v6` is best

MEASURED against `jev-latest`, N=3 passes over the same 59-fixture M2
revision plus one pass over the 45-statement M6 corpus, per variant.

| variant | agreement | margin | defect recall | no-defect recall | ECE (derived conf.) | MP-225 rate | MP-231 forward delta | all 4 ship bars |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `v4` | 81.4% | +11.9pp | 55.6% | 90.0% | 0.1127 | 1.1% | 30.0% | met |
| `v5` | 84.7% | +15.3pp | 77.8% | 80.0% | 0.1551 | 2.3% | 45.0% | met |
| `v6` | 88.1% | +18.6pp | 83.3% | 82.5% | 0.1788 | 0.0% | 45.0% | met |

All three clear the four ship-as-advisory bars from **Comparison and
Enforcement** above (margin > 0, both recalls > 0, forward delta non-zero and
above each variant's own MP-225 rate). `v6` has the best margin and the best
defect recall: it reads the two `noul` answers `v3` computed but never
consulted (`condition_is_unwanted`/`trigger_is_momentary` contradicting the
keyword's own reading), catching `EARS-FIX-048` and `050` as predicted. That
is not free: `v6` trades no-defect recall for it, flagging three more clean
rows than `v4` (33/40 against `v4`'s 36/40), and its `KeywordContradiction`
rule uses the same strict 0.5 cut `v3`/`v4`/`v5` do -- nothing about the
threshold itself tightened. `v4` remains the best of the three on bar 3.

**The calibration/accuracy tradeoff runs the other way.** `v4` -- same
labels as `v3`, only the confidence source changed -- has the best ECE
(0.1127, the only one of the three under the 0.15 M7 bound) and the worst
accuracy. `v6` has the best accuracy and the worst ECE of the three (0.1788).
Nothing here separates them: the wording and rule changes that catch more
real defects make the model's own confidence a worse predictor of whether
its call was right. None of `v4`/`v5`/`v6` clears M7's promotion bounds
(ECE < 0.15 and MP-225 at N >= 20) at the same time it clears the ship bars;
that decision is unaffected by this result, per **Interpretation** above.

**Recommendation:** ship `v6` as the advisory variant if one is picked from
this round -- highest margin, highest defect recall, same request shape as
`v3`/`v4`/`v5`. This is a recommendation for whoever wires PLAT-838 into the
skill, not itself a wiring decision; this ticket is measurement-only.

**Note (PLAT-1027):** the ECE figures above were computed with each confidence decile's midpoint in place of its mean stated confidence, which MP-226's `jev.ece-v1` names; `support/grading.rs` now uses the mean, so a rerun may report different ECEs. The recorded numbers are left as measured.
