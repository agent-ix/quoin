---
id: MP-226
title: Jev lens expected calibration error
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.expected_calibration_error
definition_version: jev.ece-v1
relationships: []
---

# Jev lens expected calibration error

## Decision Use

Decide where a lens's confidence threshold is set, and whether the confidence is worth spending.

## Population

Graded items that carry a service-reported confidence. Items without one are counted and excluded.

## Measure Definition

Expected calibration error over confidence deciles, definition `jev.ece-v1`: the occupancy-weighted mean of |bucket accuracy - bucket mean confidence|.

## Collection Procedure

Run the lens's `live-api` test through `quoin_jev::client::production`.
Retain the raw responses, question set, corpus and context revision, reported
classifier, token counts and timestamp. Dimension every observation by `lens`
and `variant`.

## Environment and Sampling

One pass is every item in the lens's labelled corpus, sent once. A transport
failure aborts the pass; it is never scored as a wrong answer.

## Interpretation

On corpora of tens of items the per-bucket counts are small; report the buckets, not only the scalar.

## Comparison and Enforcement

This plan assigns no verdict. Each lens's gate plan (MP-228, MP-229, MP-230) states its bar on this metric. Compare only like lens, variant, corpus revision and question set.
