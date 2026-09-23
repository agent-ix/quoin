---
id: MP-222
title: Jev lens margin over the constant predictor
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.constant_predictor_margin
definition_version: jev.constant-margin-v1
relationships: []
---

# Jev lens margin over the constant predictor

## Decision Use

Decide whether a lens's agreement with ground truth is evidence that it works, or only what a constant answer would also score.

## Population

Every graded item in one pass of one lens over its labelled corpus. Items the lens left unanswered or labelled outside its answer space stay in the denominator as disagreements.

## Measure Definition

Lens agreement minus the agreement of the best constant predictor, in percentage points, definition `jev.constant-margin-v1`. The constant predictor answers one fixed label per answer family, chosen to maximise agreement on this corpus. It is computed from the corpus, never written down. A contested item agrees when the answer matches any recorded reading.

## Collection Procedure

Run the lens's `live-api` test through `quoin_jev::client::production`.
Retain the raw responses, question set, corpus and context revision, reported
classifier, token counts and timestamp. Dimension every observation by `lens`
and `variant`.

## Environment and Sampling

One pass is every item in the lens's labelled corpus, sent once. A transport
failure aborts the pass; it is never scored as a wrong answer.

## Interpretation

A margin at or below zero means the lens does no better than a lookup table. A positive margin on a small corpus is weak evidence, so read it together with MP-223 and MP-224.

## Comparison and Enforcement

This plan assigns no verdict. Each lens's gate plan (MP-228, MP-229, MP-230) states its bar on this metric. Compare only like lens, variant, corpus revision and question set.
