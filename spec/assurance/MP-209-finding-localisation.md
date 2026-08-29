---
id: MP-209
title: Confirmed finding localisation rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: baseline
metric: finding_localisation_rate
definition_version: finding.localisation-v1
relationships: []
---

# Confirmed finding localisation rate

## Decision Use

Determine whether confirmed findings name the correct defect locus.

## Population

Include confirmed findings whose labels declare an expected path and, where
available, line. Keep families and languages separate.

## Measure Definition

Confirmed findings at the expected locus over confirmed findings with a labelled
locus, definition `finding.localisation-v1`.

## Collection Procedure

Retain emitted and expected loci, raw outputs, labels, scorer, tools, engine,
declaration and corpus revisions, configuration, and timestamp.

## Environment and Sampling

Use pinned labelled cases and preserve labels with no exact line as a distinct
population.

## Interpretation

Correct location does not prove the explanation or proposed remedy is correct.

## Comparison and Enforcement

Baseline per family and language. Refuse incompatible definitions and report
not-computed populations explicitly.
