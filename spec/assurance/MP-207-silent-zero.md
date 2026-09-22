---
id: MP-207
title: Benchmark silent-zero sentinel
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: sentinel.silent_zero
definition_version: benchmark.silent-zero-v1
ground_truth_kind: mechanical
subject_identity:
  name: quoin bench-tier1 benchmark instrument
  version: 0.23.1-94-g0128ed4
statistical_design:
  population: every ratio metric emitted by every completed producer in a benchmark run, excluding count metrics and genuinely empty populations
  minimum_population: 1
  sampling: exhaustive over every ratio metric in the run; no subsampling
  repetitions: 1
  estimator: count of metrics with non-zero examined, zero matched, and no diagnostic
  error_model: none -- deterministic recomputation from the same raw payload reproduces the same count
  uncertainty: none -- an exact count, not a statistical estimate
  decision_rule: gate at exactly zero; any non-zero count or missing capability is a refusal
relationships: []
---

# Benchmark silent-zero sentinel

## Decision Use

Prevent the benchmark from accepting ratio output from an instrument that read
none of a non-empty population without saying so.

## Population

Include all ratio metrics emitted by every completed producer in a benchmark
run. Exclude count metrics and genuinely empty populations.

## Measure Definition

Count metrics with non-zero examined, zero matched, and no diagnostic,
definition `benchmark.silent-zero-v1`.

## Collection Procedure

Use complete raw payloads and retain verified capabilities, tool and engine
versions, declaration and corpus digests, configuration, and timestamp.

## Environment and Sampling

Mutation tests must demonstrate rejection when the diagnostic or capability is
removed.

## Interpretation

This proves a narrow instrument-integrity property, not finding correctness.

## Comparison and Enforcement

Gate at exactly zero. Incomplete producers or missing capabilities are refusals,
not passing zeros.

## Composition

This gate decides from a single mechanical metric, so `relationships` stays
empty here. A gate that instead needs several metrics to succeed together
expresses that composition as one `references` relationship entry per
constituent metric's MeasurementPlan, not as a fabricated composite `metric`
id or as untested prose.
