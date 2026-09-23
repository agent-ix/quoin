---
id: MP-224
title: Jev lens no-defect answer recall
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.no_defect_recall
definition_version: jev.no-defect-recall-v1
relationships: []
---

# Jev lens no-defect answer recall

## Decision Use

Decide whether a lens can tell a sound item from a defective one, or whether it flags everything.

## Population

Items whose primary recorded reading is the no-defect answer: `sound` for criterion-strength, a matching pattern for EARS, `aligned` or not-vacuous for gap-analysis.

## Measure Definition

The fraction of those items the lens answered no-defect, definition `jev.no-defect-recall-v1`. A zero denominator is `not_computed`.

## Collection Procedure

Run the lens's `live-api` test through `quoin_jev::client::production`.
Retain the raw responses, question set, corpus and context revision, reported
classifier, token counts and timestamp. Dimension every observation by `lens`
and `variant`.

## Environment and Sampling

One pass is every item in the lens's labelled corpus, sent once. A transport
failure aborts the pass; it is never scored as a wrong answer.

## Interpretation

Zero here means every sound item was flagged, and each of those flags costs a human a read. This is the false-positive headline the lens tickets ask for.

## Comparison and Enforcement

This plan assigns no verdict. Each lens's gate plan (MP-228, MP-229, MP-230) states its bar on this metric. Compare only like lens, variant, corpus revision and question set.
