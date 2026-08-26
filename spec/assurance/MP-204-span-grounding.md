---
id: MP-204
title: Property span-grounding rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: observe
metric: span_grounding_rate
definition_version: property.span-grounding-v1
relationships: []
---

# Property span-grounding rate

## Decision Objective

Determine whether specific-shaped criteria expose source spans for domain,
precondition, and oracle analysis.

## Population and Scope

Include criteria classified with a specific property shape in the pinned
corpus entry.

## Measure Definition

Count records carrying all three spans over specific-shaped records, definition
`property.span-grounding-v1`. A missing producer is `not_computed`.

## Collection and Provenance

Retain the raw properties payload, tool and engine versions, definitions,
source and corpus revisions, configuration, and timestamp.

## Environment and Sampling

Keep property idioms and corpus fixed. Include known absent and present controls.

## Interpretation and Limitations

Grounded spans make analysis possible; they do not establish semantic alignment.

## Comparison and Enforcement

Remain at `observe` until a real producer and repeated records exist. No gate is
authorized.
