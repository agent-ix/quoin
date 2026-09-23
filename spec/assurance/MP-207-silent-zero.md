---
id: MP-207
title: Benchmark silent-zero sentinel
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: sentinel.silent_zero
definition_version: benchmark.silent-zero-v2
ground_truth_kind: mechanical
subject_identity:
  name: quoin bench-tier1 benchmark instrument
  version: v0.23.1
statistical_design:
  population: every ratio metric emitted by every completed producer in a benchmark run, excluding count metrics and genuinely empty populations
  sampling: exhaustive over every ratio metric in the run; no subsampling
  repetitions: 1
  estimator: count
  error_model: none -- deterministic recomputation from the same raw payload reproduces the same count
  uncertainty: none -- an exact count, not a statistical estimate
  decision_rule:
    comparator: eq
    threshold: 0
relationships: []
---

# Benchmark silent-zero sentinel

## Decision Use

Prevent the benchmark from accepting ratio output from an instrument that read
none of a non-empty population without saying so.

## Population

Include all ratio metrics emitted by every completed producer in a benchmark
run. Exclude count metrics and genuinely empty populations. This measure does
not set `statistical_design.minimum_population`: the exclusion above already
rules out an empty population, and no smaller-than-N threshold beyond that is
meaningful for an instrument-integrity sentinel.

## Measure Definition

Count metrics with non-zero examined, zero matched, and no diagnostic,
definition `benchmark.silent-zero-v2`. `benchmark.silent-zero-v1` is the same
count; v2 is the version at which `statistical_design.estimator` and
`.decision_rule` became engineering-assurance's typed form (`count`, and
`eq` against threshold 0) instead of prose, and a changed definition member
takes a new version.

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

The typed `decision_rule` covers the first sentence's count. The
missing-capability refusal is a separate condition and is **not** evaluated by
`quoin measurement verify` (FR-108): no decision-rule field can state it, and
the checker reads the plan's rule, not this prose. It is checked by reading a
collection's `verificationStack.capabilities`, and an `accept` from the
verifier says nothing about it.

## Composition

This gate decides from a single mechanical metric, so `relationships` stays
empty here. A gate that instead needs several metrics to succeed together
expresses that composition as one `references` relationship entry per
constituent metric's MeasurementPlan, not as a fabricated composite `metric`
id or as untested prose.
