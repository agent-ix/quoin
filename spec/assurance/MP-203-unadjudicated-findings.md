---
id: MP-203
title: Unadjudicated advisory finding count
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: finding_precision.unadjudicated
definition_version: finding.unadjudicated-v1
relationships: []
---

# Unadjudicated advisory finding count

## Decision Use

Make visible how many advisory firings the corpus has not ruled correct or
incorrect.

## Population

Include advisory findings governed by neither a case/standing ruling nor an
exact retained ruling compatible with `finding.precision.advisory-v1`, per
family. Ambiguous and unresolved exact rulings stay in this population.

## Measure Definition

Count unadjudicated findings, definition `finding.unadjudicated-v1`. It is a
count, not a precision estimate.

## Collection Procedure

Retain normalized finding envelopes, content digests, expectation digests,
raw output, scorer and corpus revisions, tool configuration, timestamp, rubric
version, reviewer, rationale, disagreements, and defect follow-up. Refuse a
ruling whose finding bytes, metric version, rubric version, or population
identity changed.

## Environment and Sampling

Use the complete pinned controlled corpus and fixed adjudication rules. A
historical adjudication keeps its own source-report digest; it is never silently
re-applied to a different normalized finding.

## Interpretation

A lower count can mean adjudication or fewer firings; report both. Zero says
the corpus ruled on its firings, not that the detector is correct. Correct and
incorrect dispositions enter precision; ambiguous and unresolved dispositions
do not.

## Comparison and Enforcement

Ratchet the count per family while separately reporting emitted findings and
adjudication changes.
