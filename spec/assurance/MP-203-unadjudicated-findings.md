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

## Decision Objective

Make visible how many advisory firings the corpus has not ruled correct or
incorrect.

## Population and Scope

Include advisory findings whose case expectation names the reason in neither
the required nor absent set, per family.

## Measure Definition

Count unadjudicated findings, definition `finding.unadjudicated-v1`. It is a
count, not a precision estimate.

## Collection and Provenance

Retain finding loci, expectation digests, raw output, scorer and corpus
revisions, tool configuration, and timestamp.

## Environment and Sampling

Use the complete pinned controlled corpus and fixed adjudication rules.

## Interpretation and Limitations

A lower count can mean adjudication or fewer firings; report both. Zero says
the corpus ruled on its firings, not that the detector is correct.

## Comparison and Enforcement

Ratchet the count per family while separately reporting emitted findings and
adjudication changes.
