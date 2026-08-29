---
id: MP-204
title: Property span-grounding rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: span_grounding_rate
definition_version: property.span-grounding-v1
relationships: []
---

# Property span-grounding rate

## Decision Use

Determine whether specific-shaped criteria expose source spans for domain,
precondition, and oracle analysis.

## Population

Include criteria classified with a specific property shape in the pinned
corpus entry.

## Measure Definition

Count records carrying all three spans over records classified as round-trip,
idempotence, ordering, invariant, error-case, lifecycle, or concurrency,
definition `property.span-grounding-v1`. A missing producer is `not_computed`.
For a specific-shaped record, an absent key is `missing`, a null span is
`unavailable`, and an invalid span object is `malformed`; all remain named
denominator misses. Non-specific property shapes are retained as
`not_applicable` exclusions.

## Collection Procedure

Retain the raw properties payload, tool and engine versions, definitions,
source and corpus revisions, configuration, and timestamp.

## Environment and Sampling

Keep property idioms and corpus fixed. Include known absent and present controls.

## Interpretation

Grounded spans make analysis possible; they do not establish semantic alignment.

## Comparison and Enforcement

The retained Tier-1 baseline is a one-way non-regression ratchet. Compare only
like corpus, declaration, and population inputs.
