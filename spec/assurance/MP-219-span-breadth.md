---
id: MP-219
title: Broad real-repository span grounding
type: MeasurementPlan
status: active
owner: quoin quality
stage: gate
metric: span_breadth_rate
definition_version: property.span-breadth-v2
ground_truth_kind: agent-labelled
statistical_design:
  population: the frozen labeled population of 40 unique normalized criteria across eight property shapes and two real repositories (quoin and quire-rs)
  sampling: none; the canonical stack replays every label and duplicates cannot inflate the denominator
  repetitions: 1
  estimator: proportion
  population_set:
    ids: [quire-rs-FR-002-AC-2-2ae46bce99, quire-rs-FR-002-AC-3-dbb547e8b8, quire-rs-FR-002-AC-4-1c81f1a9a5, quire-rs-FR-004-AC-2-e352dfdefb, quire-rs-FR-011-AC-15-3fc85d408c, quire-rs-FR-011-AC-16-0db532dac8, quire-rs-FR-011-AC-21-7c4830ee91, quire-rs-FR-011-AC-6-710aa03a0c, quire-rs-FR-014-AC-1-831757d3c7, quire-rs-FR-014-AC-5-c2f16a8fe5, quire-rs-FR-022-AC-6-888f9deb29, quire-rs-FR-039-AC-5-1dd9304d49, quire-rs-FR-050-AC-14-3b8b7b2477, quire-rs-FR-050-AC-18-85ad5f0054, quire-rs-FR-050-AC-19-4dd16e775c, quire-rs-FR-050-AC-23-eb4976d1b7, quire-rs-FR-052-AC-16-2cbf3263a7, quire-rs-FR-063-AC-2-daccd44754, quire-rs-NFR-020-AC-2-f0663d1fab, quire-rs-StR-006-VC-4-9983d7b2c5, quoin-FR-001-AC-3-920d0c3552, quoin-FR-002-AC-3-aed0a0e503, quoin-FR-003-AC-1-865884c064, quoin-FR-005-AC-1-786fc6615d, quoin-FR-013-AC-4-f0f645ad39, quoin-FR-014-AC-3-105ebc382c, quoin-FR-025-AC-8-e47ab28daa, quoin-FR-028-AC-3-0a3904a4a2, quoin-FR-028-AC-8-d2d3238c64, quoin-FR-029-AC-12-4e0d1f4732, quoin-FR-029-AC-14-5d6f45d3cf, quoin-FR-030-AC-13-a7c4298e61, quoin-FR-030-AC-3-28682eefb9, quoin-FR-030-AC-8-8154829c83, quoin-FR-031-AC-23-a69def5430, quoin-FR-031-AC-9-297fc9793c, quoin-FR-048-AC-2-20af68ffdb, quoin-FR-052-AC-5-31fdf1fc5b, quoin-FR-053-AC-6-1f126c0730, quoin-NFR-023-AC-4-7fa2bc4a1d]
  error_model: a label is wrong or a repository drifts from the frozen set
  uncertainty: none estimated; the population is the full frozen label set and any wrong or unsafe span blocks promotion
  decision_rule:
    comparator: ge
    threshold: 1.0
protected_apparatus:
  - bench/span-breadth-v1-labels.json
negative_controls:
  - kind: apparatus-edit
    description: the frozen label set is protected apparatus; relabelling without a new definition version rejects
  - kind: suppressed-observation
    description: every label is replayed; there is no runtime sample, so a failing label cannot be left out of the denominator
relationships: []
---

# Broad real-repository span grounding

## Decision Use

Decide whether span grounding is useful beyond duplicated refusal fixtures.

## Population

The plan's `population_set` is the id of every label in `bench/span-breadth-v1-labels.json`; the `tc_3006` test fails when the two differ.

The label file holds 40 unique normalized criteria: 20 supported exact-grounding outcomes and 20 justified refusals, across eight property shapes and two real repositories (quoin and quire-rs). The earlier "at least 60 criteria in three repositories" described the corpus before it was reduced to the repositories' own specs; the file is the source of truth and the 40-label count is what this plan holds.

The producer is the `quoin-cli` test `tc_3301` (`tests/nfr_populations/span_breadth.rs`). It reads the ids from the label file, replays each label through `quoin_quire::properties::classify` in the checkout of the repository the label names, and states every label as `examined_ids` and those whose reviewed outcome the classifier reproduces as `matched_ids`. It needs a quire-rs checkout (`QUIRE_RS_CHECKOUT`) and is not part of the default gate; without the checkout the replay refuses instead of examining a subset. The default gate runs the replay logic over a fixture repository (`tc_3300`) and does not claim a measurement.

## Measure Definition

Exact reviewed boundaries or explicit justified safe refusals divided by the frozen labeled population.

## Collection Procedure

Every label names repository, document, row, statement, property, expected outcome, and review rationale.

## Environment and Sampling

The canonical stack replays all labels; there is no runtime sample and duplicates cannot inflate the denominator.

## Interpretation

Challenge labels cover truncation, overbreadth, wrong-subject, hyphenated, nested, coordinated, and refusal risks. They do not claim all prose is decomposable.

## Comparison and Enforcement

Any wrong/unsafe span, missing label, repository drift, or breadth-floor failure blocks promotion.
