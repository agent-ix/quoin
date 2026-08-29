---
id: MP-208
title: Minting section-hit rate
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: minting.section_hit_rate
definition_version: minting.section-hit-v1
relationships: []
---

# Minting section-hit rate

## Decision Objective

Show whether trace-target declarations find the document sections they claim to
read.

## Population and Scope

Include trace-target and minting-document pairs in scored controlled cases;
exclude pending cases visibly.

## Measure Definition

Sum matched over examined pairs, definition `minting.section-hit-v1`, retaining
case, mode, and language dimensions.

## Collection and Provenance

Retain raw coverage payloads, case inventory, declaration digest, engine,
scorer and corpus revisions, configuration, and timestamp.

## Environment and Sampling

Use the pinned corpus and declared modules; include absent-section controls.

## Interpretation and Limitations

Finding a section does not establish that the correct table or ids were minted.

## Comparison and Enforcement

Compare branches only under like definitions and populations. Population
changes are flagged, not rendered as bare regressions.
