---
id: MP-201
title: Per-family finding precision
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: finding_precision
definition_version: finding.precision-v1
relationships: []
---

# Per-family finding precision

## Decision Use

Determine whether a change makes reported findings more or less correct within
each labelled defect family.

## Population

Include findings emitted for one family over the pinned labelled corpus. Keep
families separate and report unadjudicated findings.

## Measure Definition

`true_positives / (true_positives + false_positives)`, definition
`finding.precision-v1`; a zero denominator is `not_computed`, never zero.

## Collection Procedure

Retain finding loci, labels, raw outputs, scorer revision, tools, engine,
declaration digest, corpus revision, configuration, and timestamp.

## Environment and Sampling

Use the pinned corpus and unchanged label set. Live-repository samples are
reported separately with adjudication coverage.

## Interpretation

Corpus precision does not estimate live precision when the live population is
unadjudicated. A correct-family wrong-location finding is false positive.

## Comparison and Enforcement

Compare branches only for like definitions, configurations, labels, and
populations. This plan assigns no universal target or verdict.
