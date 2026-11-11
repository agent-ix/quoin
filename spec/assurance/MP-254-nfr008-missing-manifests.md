---
id: MP-254
title: Candidates without a manifest are skipped
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: nfr008.missing_manifest_skip_rate
definition_version: nfr008.missing-manifest-skip-v1
ground_truth_kind: mechanical
statistical_design:
  population: every kind of candidate path that holds no `manifest.yaml` the producer builds
  sampling: complete census of the missing-manifest kinds in the producer's table
  repetitions: 1
  estimator: proportion
  population_set:
    ids: [nonexistent-path, empty-directory, directory-without-manifest, regular-file]
  error_model: none; each id is probed once against the real binary and the outcome is read directly from its exit status and streams
  uncertainty: none; a complete census of the declared set, not a sample
  decision_rule:
    comparator: ge
    threshold: 1.0
negative_controls:
  - kind: suppressed-observation
    description: an id left out of examined_ids is refused as population_partial, so a producer cannot pass by probing a convenient subset
relationships:
  - target: ix://agent-ix/quoin/NFR-008
    type: measures
  - target: ix://agent-ix/quoin/FR-131
    type: references
---

# Candidates without a manifest are skipped

Measures NFR-008-M-2: missing manifests that are skipped rather than aborting
assembly.

## Decision Use

Decide whether a path that is not a module is ignored, so a stray directory does
not break every catalog read.

## Population

A nonexistent path, an empty directory, a directory holding other files but no
manifest, and a regular file. The producer owns the table that builds them and
the `tc_3005` test compares its ids with this plan's set.

## Measure Definition

For each kind the producer lists the candidate ahead of a valid module in
`QUOIN_MODULE_PATHS` and runs `quoin catalog list --json`. The kind matches when
the process exits zero and the valid module is in the catalog. Value: matching
kinds over kinds examined, definition `nfr008.missing-manifest-skip-v1`.

## Collection Procedure

As MP-253, in `tc_3003_nfr_populations.rs`.

## Interpretation

100% means each kind is skipped and does not hide the modules after it.

