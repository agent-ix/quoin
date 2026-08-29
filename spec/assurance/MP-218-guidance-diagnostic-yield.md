---
id: MP-218
title: Executable diagnostic guidance yield
type: MeasurementPlan
status: active
owner: quoin quality
stage: gate
metric: guidance.diagnostic_yield
definition_version: guidance.diagnostic-yield-v1
relationships: []
---

# Executable diagnostic guidance yield

## Decision Use

Decide whether each reviewed diagnostic step produces evidence that distinguishes the relevant causes.

## Population

Selected reviewed records whose structured next move is `diagnostic`.

## Measure Definition

Steps whose canonical replay yields the cited causal channel and a cause-distinguishing observation, divided by reviewed diagnostic records.

## Collection Procedure

Retained raw findings, action-template digests, evaluator contract, and stack attestation identify every observation.

## Environment and Sampling

The same deterministic partition sampling used for correctness applies.

## Interpretation

Yield does not authorize an automatic repair; it proves the next step is informative.

## Comparison and Enforcement

Exactly 100% is required for promotion.
