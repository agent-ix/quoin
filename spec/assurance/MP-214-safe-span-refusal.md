---
id: MP-214
title: Safe property-span refusal
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: span_safe_refusal_rate
definition_version: property.safe-refusal-v1
relationships: []
---

# Safe property-span refusal

## Decision Use

Determine whether the producer declines unsupported decomposition explicitly
instead of emitting a precise-looking wrong span.

## Population

Include controlled criteria labelled with an expected structured refusal
reason. This denominator is separate from span presence and correctness.

## Measure Definition

Count a safe refusal when all three spans are absent and the record carries the
expected refusal signal. An absent span without that reason is an
`unjustified-refusal`; any emitted span is an `unsafe-emission`. Definition
`property.safe-refusal-v1`.

## Collection Procedure

Retain expected and observed reasons, raw producer payload, producer version,
named misses, source revision, and timestamp.

## Environment and Sampling

Keep the refusal fixture, declaration modules, and producer fixed. Report the
wrong-span, unexpected-refusal, safe-refusal, and unsafe-emission counts beside
one another.

## Interpretation

A safe refusal preserves uncertainty; it does not improve span presence and is
not counted as correctness on a case whose expected loci are known.

## Comparison and Enforcement

Overall and per-family rates are one-way non-regression ratchets. Compare only
like controlled-label definitions and producer inputs.
