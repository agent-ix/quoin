---
id: MP-213
title: Property span correctness
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: span_correctness_rate
definition_version: property.span-correctness-v1
relationships: []
---

# Property span correctness

## Decision Use

Determine whether emitted property spans identify the intended subject and
clause boundaries, independently of whether spans are merely present.

## Population

Include controlled criteria carrying exact expected domain, precondition, and
oracle loci. The fixture covers an exact positive, hyphen-boundary errors,
post-predicate filter errors, and a partitive wrong-subject error.

## Measure Definition

Count a criterion only when every emitted span text and statement-relative
start/end coordinate exactly equals its expected locus. Expected null and
observed null agree for a clause the criterion does not contain. A present but
different span is `wrong-span`; no spans where loci are expected is
`unexpected-refusal`. Definition `property.span-correctness-v1`.

## Collection Procedure

Retain the controlled labels, raw `quire properties --json` payload, producer
version, named expected-versus-observed misses, source revision, and timestamp.

## Environment and Sampling

Keep the fixture, declaration modules, and producer fixed. Report overall and
per failure-shape family; do not average families.

## Interpretation

This controlled set is a regression contract, not an estimate of ecosystem
precision. Presence alone earns no correctness credit.

## Comparison and Enforcement

Overall and per-family rates are one-way non-regression ratchets. Compare only
like controlled-label definitions and producer inputs.
