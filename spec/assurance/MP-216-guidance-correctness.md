---
id: MP-216
title: Independent action-guidance correctness
type: MeasurementPlan
status: active
owner: quoin quality
stage: gate
metric: guidance.correctness
definition_version: guidance.correctness-v1
relationships: []
---

# Independent action-guidance correctness

## Decision Use

Decide whether structured guidance is correct, rather than merely populated.

## Population

Every record in partitions of five or fewer; otherwise a deterministic five-record content-hash sample plus every distinct action template.

## Measure Definition

Passing independent reviews divided by selected applicable records. The target is 100%.

## Collection Procedure

The evaluator contract, review evidence, stack lock, producer records, and their digests are retained together.

## Environment and Sampling

Sampling is deterministic and consumer-owned. Producers cannot exclude their own missing fields.

## Interpretation

This review judges the recommendation against retained evidence; repair and diagnostic execution are measured separately.

## Comparison and Enforcement

Any miss blocks promotion. Contract or partition movement requires a new metric version.
