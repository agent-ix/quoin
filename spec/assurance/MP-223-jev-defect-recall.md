---
id: MP-223
title: Jev lens defect recall
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.defect_recall
definition_version: jev.defect-recall-v1
relationships: []
---

# Jev lens defect recall

## Decision Use

Decide whether a lens finds the defects it exists to find.

## Population

Items where no recorded reading is the no-defect answer. Items with a contested no-defect reading are excluded, because answering no-defect there is defensible.

## Measure Definition

The fraction of those items the lens answered with any defect label, definition `jev.defect-recall-v1`. A wrong defect label still counts as found. A zero denominator is `not_computed`, never zero.

## Collection Procedure

Run the lens's `live-api` test through `quoin_jev::client::production`.
Retain the raw responses, question set, corpus and context revision, reported
classifier, token counts and timestamp. Dimension every observation by `lens`
and `variant`.

## Environment and Sampling

One pass is every item in the lens's labelled corpus, sent once. A transport
failure aborts the pass; it is never scored as a wrong answer.

## Interpretation

High recall with low MP-224 is the flag-everything failure.

## Comparison and Enforcement

This plan assigns no verdict. Each lens's gate plan (MP-228, MP-229, MP-230) states its bar on this metric. Compare only like lens, variant, corpus revision and question set.
