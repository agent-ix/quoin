---
id: MP-231
title: Jev EARS engine/semantic delta rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.ears_engine_semantic_delta
definition_version: jev.ears-delta-v3
relationships: []
---

# Jev EARS engine/semantic delta rate

## Decision Use

Decide whether the EARS semantic lens finds anything the deterministic engine
(quire-rs FR-042, `iso-spec-core` grammar) does not already find. PLAT-838
names this the justifying number: near zero means the lens adds nothing over
the grammar engine and should not ship.

## Population

Real FR/NFR/StR statements drawn from spec trees under `~/dev` (not the
labelled M2 fixture set), each paired with the actual verdict of running
`quire validate --scope . "spec/**/*.md"` against that statement's own repo at
measurement time -- never an imitation of the engine's grammar. Two subsets:
statements the engine's `[ears:*]` check left the owning document fully
`[grammar]`-clean, and statements the engine flagged with one or more
`ears:*` codes.

## Measure Definition

Two rates. **Two definition versions exist, and an observation states which
one it used; they are never compared to each other as if they were the same
number.**

### `jev.ears-delta-v1` -- the original, six-way-choice rule

- **Forward delta**: of the engine-clean statements, the fraction where Jev's
  `ears_pattern_actual` choice disagrees with the pattern the statement's own
  trigger keyword implies (`When`→`event_driven`, `While`→`state_driven`,
  `If…then`→`unwanted_behaviour`, `Where`→`optional_feature`, no trigger→
  `ubiquitous`), OR Jev's `response_measurable` noul reads false, OR
  `condition_is_unwanted`/`trigger_is_momentary` contradict the keyword's own
  implied reading. This is the number PLAT-838 calls the delta.
- **Inverse delta**: of the engine-flagged statements, the fraction where Jev's
  answers give no reason to treat the statement as defective (candidate
  engine false positives, reported separately and never netted against the
  forward delta).

### `jev.ears-delta-v3` -- the noul-derived rule

Pre-registered here before the first live call that uses it. MP-229's bar 4
reads this version whenever the verdict it feeds was reached under the `v3`
classification rule, because a verdict must not mix a `v1` delta into a `v3`
decision.

Same two populations and the same two directions, but the flag is the `v3`
derivation rule (`tests/support/ears.rs::derive_defect_from_noul`), the
identical function the M2 grading uses, applied to the statement's own
engine-derived pattern:

- a statement is flagged when Jev's `response_measurable` noul reads below
  0.5; or
- when the engine's naive pattern is `event_driven` and either
  `condition_is_unwanted` reads at or above 0.5, or `trigger_is_momentary`
  reads below 0.5.

Jev's six-way `ears_pattern_actual` choice is **not consulted**. It is still
asked and still recorded, so the `v1` rate is computable from the same
responses and is reported beside the `v3` rate as a comparison.

A statement is counted in the denominator only where the service answered at
least one noul this rule consults; an unanswered statement is excluded, never
scored as agreement.

**Stated asymmetry, before running.** Because both disambiguators are phrased
for a `When` statement, `jev.ears-delta-v3` can raise a pattern-confusion
flag only on engine-`event_driven` statements. On any other engine pattern
only the measurability branch can fire. A `v3` forward delta is therefore
expected to be at or below the `v1` forward delta on the same corpus, and a
lower number here is not by itself evidence the lens got better or worse --
it is a fact about which questions the rule reads.

A zero denominator on either side is `not_computed`, never zero.

## Collection Procedure

Run the lens's `live-api` test through `quoin_jev::client::production`. Retain
the raw responses, question set, the corpus (including which repo and commit
each statement came from and the `quire --version` string used to grade it),
reported classifier, token counts and timestamp. Dimension every observation
by `lens: ears` and `variant`.

## Environment and Sampling

One pass sends every corpus item once. State the repos, the commit each was
read at, and the count in each subset. A transport failure aborts the pass; it
is never scored as a wrong answer.

## Interpretation

This is a fact about overlap between two independent classifiers, not an
accuracy score against ground truth -- MP-222/MP-223/MP-224 (run over the
separate labelled M2 corpus) carry that job. A high forward delta only shows
the lens disagrees with the engine's keyword parse; whether Jev's reading is
*right* is what M2's agreement and recall numbers are for. Report both
directions; a lens that only ever disagrees in the engine's favor (zero
inverse delta) is indistinguishable from a lens with no independent judgment
of its own.

## Measured values, 2026-09-22

MEASURED at `variant: v3`, one pass over the 45-statement M6 corpus, graded
both ways from the identical responses. Record:
`spec/evidence/measurements/plat838-ears-v3-20260922T0420Z.json`.

| direction | `jev.ears-delta-v3` | `jev.ears-delta-v1` | denominator |
| --- | --- | --- | --- |
| forward (engine-clean, Jev flags) | **30.0%** (6/20) | 45.0% (9/20) | 20/20 answered |
| inverse (engine-flagged, Jev calls clean) | **88.0%** (22/25) | 12.0% (3/25) | 25/25 answered |

The forward drop from 45.0% to 30.0% is the asymmetry this plan stated
before running, not a change in the lens: `v3` cannot raise a
pattern-confusion flag outside an engine-`event_driven` reading.

The inverse figure inverts between the two versions, and the `v3` value
carries a reading this plan's **Interpretation** does not otherwise supply:
88.0% is **not** a candidate-engine-false-positive rate. The `v3` rule has no
question that could corroborate `ears:missing-subject`, `ears:non-singular`,
`ears:unclassifiable` or `ears:non-canonical-trigger`, which are the codes
this pool is flagged under, so it calls those statements clean because it
cannot see what the engine saw -- not because the engine was wrong. Quoting
this number as an engine defect rate is a misreport. It remains barred, per
**Comparison and Enforcement**.

An earlier `v1`-only measurement of this metric is recorded at
`spec/evidence/measurements/plat838-ears-v1-20260922T011531Z.json` (forward
40.0% over a 20-statement pool). It was fed into a `v3` verdict, which
MP-229 now forbids.

## Comparison and Enforcement

This plan assigns no verdict. MP-229 (the EARS gate plan) states the bar on
this metric. Compare only like lens, variant, corpus revision, question set,
and `quire` engine version.
