---
id: MP-225
title: Jev lens disagreement under repetition
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.disagreement_rate
definition_version: jev.disagreement-v1
relationships: []
---

# Jev lens disagreement under repetition

## Decision Use

Decide whether a lens's output may be treated as a fact rather than an opinion.

## Population

N repeated passes over a byte-identical corpus, context and question set. N is stated with every observation.

## Measure Definition

Mean over all pairs of passes of the fraction of items whose compared class changed, definition `jev.disagreement-v1`. An item present in one pass and absent from another counts as changed.

## Collection Procedure

Run the lens's `live-api` test through `quoin_jev::client::production`.
Retain the raw responses, question set, corpus and context revision, reported
classifier, token counts and timestamp. Dimension every observation by `lens`
and `variant`.

## Environment and Sampling

One pass is every item in the lens's labelled corpus, sent once. A transport
failure aborts the pass; it is never scored as a wrong answer.

## Interpretation

The ad-hoc LLM pass measured 17.0% at N=12 (quire-rs PR #482). That figure is a reference, not a bar.

## Comparison and Enforcement

This plan assigns no verdict. Each lens's gate plan (MP-228, MP-229, MP-230) states its bar on this metric. Compare only like lens, variant, corpus revision and question set.
