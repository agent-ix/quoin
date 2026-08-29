---
id: MP-217
title: Executable guidance repair success
type: MeasurementPlan
status: active
owner: quoin quality
stage: gate
metric: guidance.repair_success
definition_version: guidance.repair-success-v1
relationships: []
---

# Executable guidance repair success

## Decision Use

Decide whether each reviewed remedy actually removes its finding without breaking the controlled verification.

## Population

Selected reviewed records whose structured next move is `remedy`.

## Measure Definition

Remedies whose failure fixture reports the family, repaired control does not, and corpus verification remains green, divided by reviewed remedy records.

## Collection Procedure

The exact failure/control identities and producer stack are retained in review evidence and measurement v2.

## Environment and Sampling

The canonical QA and Tier-1 replays use the immutable verification-stack lock.

## Interpretation

The proof establishes the banked repair, not every possible user edit.

## Comparison and Enforcement

Exactly 100% is required for promotion.
