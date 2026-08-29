---
id: MP-205
title: Finding actionability rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: baseline
metric: actionability_rate
definition_version: finding.actionability-v1
relationships: []
---

# Finding actionability rate

## Decision Objective

Determine whether emitted findings identify enough location information for a
reader to begin a repair.

## Population and Scope

Include every finding emitted for one corpus entry and retain family and source
dimensions.

## Measure Definition

Findings naming a row id or document line over emitted findings, definition
`finding.actionability-v1`; store both counts.

## Collection and Provenance

Retain each finding and locus, raw output, scorer, tools, engine, corpus and
source revisions, configuration, and timestamp.

## Environment and Sampling

Report tier-1 and live/battletest populations separately.

## Interpretation and Limitations

A locus is necessary but not sufficient: it does not prove the explanation or
remediation is correct. Use L3 corpus assertions for that stronger claim.

## Comparison and Enforcement

Baseline by population. Refuse unlike definitions/configurations and flag
population changes; no universal threshold is declared.
