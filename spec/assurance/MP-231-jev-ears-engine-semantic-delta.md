---
id: MP-231
title: Jev EARS engine/semantic delta rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.ears_engine_semantic_delta
definition_version: jev.ears-delta-v1
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

Two rates, both `jev.ears-delta-v1`:

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

## Comparison and Enforcement

This plan assigns no verdict. MP-229 (the EARS gate plan) states the bar on
this metric. Compare only like lens, variant, corpus revision, question set,
and `quire` engine version.
