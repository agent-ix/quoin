---
id: MP-206
title: Cost per confirmed insight
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: observe
metric: cost_per_confirmed_insight
definition_version: finding.confirmed-cost-v1
relationships: []
---

# Cost per confirmed insight

## Decision Use

Describe the tool calls and model tokens spent for each confirmed true positive.

## Population

Include completed runs with confirmed findings. Keep agent and deterministic
benchmark environments separate.

## Measure Definition

Report tool calls and model tokens divided by confirmed true positives,
definition `finding.confirmed-cost-v1`. Unobserved tokens and zero confirmations
are `not_computed`, never zero.

## Collection Procedure

Retain harness/model identity, tool calls, token envelope, findings and rulings,
scope, revisions, configuration, raw evidence, and timestamp.

## Environment and Sampling

Record model, context, host, corpus, and task shape because they confound cost.

## Interpretation

Lower cost is not better if recall or correctness falls. Do not use this metric
for individual performance assessment.

## Comparison and Enforcement

Remain observational until comparable repeated runs exist. No target or gate is
authorized.
