---
id: MP-219
title: Broad real-repository span grounding
type: MeasurementPlan
status: active
owner: quoin quality
stage: gate
metric: span_breadth_rate
definition_version: property.span-breadth-v1
relationships: []
---

# Broad real-repository span grounding

## Decision Use

Decide whether span grounding is useful beyond duplicated refusal fixtures.

## Population

At least 60 unique normalized criteria, five property shapes, three real repositories, and 20 supported exact-grounding outcomes.

## Measure Definition

Exact reviewed boundaries or explicit justified safe refusals divided by the frozen labeled population.

## Collection Procedure

Every label names repository, full revision, document, row, statement, property, expected outcome, and review rationale. The verification-stack lock hashes the label set and executable.

## Environment and Sampling

The canonical stack replays all labels; there is no runtime sample and duplicates cannot inflate the denominator.

## Interpretation

Challenge labels cover truncation, overbreadth, wrong-subject, hyphenated, nested, coordinated, and refusal risks. They do not claim all prose is decomposable.

## Comparison and Enforcement

Any wrong/unsafe span, missing label, repository drift, or breadth-floor failure blocks promotion.
