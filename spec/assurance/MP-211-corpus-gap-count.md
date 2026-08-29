---
id: MP-211
title: Controlled-corpus bounds gap count
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: bounds.gap_count
definition_version: bounds.gap-count-v1
relationships: []
---

# Controlled-corpus bounds gap count

## Decision Use

Keep unrepresented corpus cells visible beside every detection-recall score.

## Population

Adopt every declared bounds cell in the pinned `qa-corpus`.

## Measure Definition

Adopt `qa-corpus/assurance/MP-201-gap-count.md` definition
`bounds.gap-count-v1`: count cells in state GAP, never a ratio.

## Collection Procedure

Tier 1 derives the count from `bounds.py --json`, retains the raw matrix and
repeats the count as a typed observation beside each recall partition.

## Environment and Sampling

Use the complete validated inventory. Missing or unreadable cases fail before
measurement.

## Interpretation

A lower count means fewer declared holes, not better detection. Out-of-scope is
a reviewed disposition carrying a written reason.

## Comparison and Enforcement

Ratchet only under an unchanged bounds definition. Corpus population changes
require deliberate re-baselining.
