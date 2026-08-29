---
id: MP-202
title: Per-family detection recall
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: ratchet
metric: finding_recall
definition_version: finding.recall-v1
relationships: []
---

# Per-family detection recall

## Decision Use

Expose defect families, levels, modes, or languages that the toolchain cannot
detect so a green aggregate cannot hide a hole.

## Population

Include labelled, findable seeded defects in the pinned corpus. Preserve family,
detection level, mode, and language dimensions; exclusions remain visible.

## Measure Definition

`true_positives / (true_positives + misses)`, definition `finding.recall-v1`,
reported per family and never macro-averaged.

## Collection Procedure

Retain case and control identities, labels, raw outputs, scorer, tool and engine
versions, declaration digest, corpus revision, configuration, and timestamp.

## Environment and Sampling

Run the complete controlled matrix and publish `bounds.gap_count` beside recall.

## Interpretation

Perfect recall can come from firing on everything; healthy-control precision is
required. A GAP is unmeasured, not a miss or a pass.

## Comparison and Enforcement

Ratchet only comparable populated cells. A zero-recall family remains explicit;
no average or severity verdict is emitted by the comparison layer.
