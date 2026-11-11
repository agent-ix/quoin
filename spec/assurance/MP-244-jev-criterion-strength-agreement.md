---
id: MP-244
title: Jev criterion-strength agreement over the constant predictor
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: jev.criterion_strength.agreement
definition_version: jev.criterion-strength-agreement-v1
ground_truth_kind: agent-labelled
statistical_design:
  population: every graded item of one pass of the criterion-strength lens over the fifteen-fixture corpus
  sampling: exhaustive; one pass, every fixture sent once
  minimum_population: 15
  repetitions: 1
  estimator: proportion
  population_set:
    ids: [CS-FIX-001, CS-FIX-002, CS-FIX-003, CS-FIX-004, CS-FIX-005, CS-FIX-006, CS-FIX-007, CS-FIX-008, CS-FIX-009, CS-FIX-010, CS-FIX-011, CS-FIX-012, CS-FIX-013, CS-FIX-014, CS-FIX-015]
  error_model: agent-labelled ground truth; a lens answer is noisy across passes, measured by MP-225
  uncertainty: fifteen fixtures is a small corpus and one pass can beat the constant by luck; MP-228 states the repeated-pass condition
  decision_rule:
    comparator: gt
    baseline: constant-predictor
  answer_families:
    - weakness_kind
    - coverage
protected_apparatus:
  - skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fixtures.json
  - skills/spec-criterion-strength-analysis/assets/fixtures/criterion-strength-fr-context.json
  - skills/spec-criterion-strength-analysis/assets/question-set.json
negative_controls:
  - kind: apparatus-edit
    description: the fixture corpus, its FR-context sidecar and the question set are protected apparatus; an edit to any of them without a new definition version rejects
  - kind: suppressed-observation
    description: the item rows must number exactly the aggregate's examined, so a fixture the lens fails to answer cannot be quietly left out of the denominator
relationships: []
---

# Jev criterion-strength agreement over the constant predictor

## Decision Use

Decide whether one graded pass of the criterion-strength lens agrees with the corpus more often than the best constant predictor (MP-222, bar 1 of MP-228).

## Population

The plan's `population_set` is the `fixture_id` of every fixture in `criterion-strength-fixtures.json`; the `tc_3006` test fails when the two differ, and `graded_corpus_collection` states them as the aggregate's `examined_ids` and `matched_ids`.

The fifteen fixtures of `criterion-strength-fixtures.json`: eleven `weakness_kind` criteria and four `coverage` FRs. An unanswered or unrecognized row stays in the denominator as a disagreement.

## Measure Definition

The proportion of graded items on which the lens returned the primary or a contested reading, definition `jev.criterion-strength-agreement-v1`. The constant predictor is recomputed from the item rows `quoin_jev::graded_corpus_collection` stores beside the aggregate, one fixed label per answer family.

## Collection Procedure

Run `the_lens_beats_the_do_nothing_baseline` in `rust/crates/quoin-jev/tests/live_criterion_strength.rs` with `--features live-api` and `JEV_MEASUREMENT_REPO` naming this repository. The test stores the collection before any assertion.

## Environment and Sampling

One pass of one variant, every fixture sent once. A transport failure aborts the pass and is never scored.

## Interpretation

A pass that beats the constant on this corpus is weak evidence on its own. MP-228 decides GO from this bar together with defect recall, sound recall and the repeated passes.

## Comparison and Enforcement

`quoin measurement verify` accepts when the stored agreement rate is strictly above the recomputed constant-predictor baseline. This plan does not replace MP-228's other bars.
