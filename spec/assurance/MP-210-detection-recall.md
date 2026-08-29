---
id: MP-210
title: Controlled-corpus detection recall
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: detection.recall
definition_version: detection-recall-v1
relationships: []
---

# Controlled-corpus detection recall

## Decision Use

Prevent a strong aggregate from hiding a blind grading level, failure mode,
language, or finding family in the validator toolchain.

## Population

Use every findable failure case in the pinned `qa-corpus`. Report L1, L2, and
L3 independently for every mode-language-family partition.

## Measure Definition

Adopt `qa-corpus/assurance/MP-202-detection-recall.md` definition
`detection-recall-v1`. Store reached and examined counts, exact missed case ids,
and the adjacent corpus GAP count. Separately retain every L2/L3 miss with its
producer, expected and observed locus, structural root cause, and disposition.

## Collection Procedure

The Tier-1 producer records engine, corpus, declaration and scorer revisions,
the complete raw report, and one typed observation per family partition. The
raw report's `locality_miss_inventory` is the named repair inventory referenced
by the aggregate rendering.

## Environment and Sampling

Run every static case with the explicitly named Quire binary and vendored
declarations. Do not sample or average partitions.

## Interpretation

Recall does not establish precision; controlled pairs and their differential
gate constrain false positives. A zero is a measured miss list, never an absent
producer.

## Comparison and Enforcement

Lower recall fails. Improved recall requires the explicit Tier-1 update to
retain the tighter baseline. Population changes are incomparable.
