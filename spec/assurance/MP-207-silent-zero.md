---
id: MP-207
title: Benchmark silent-zero sentinel
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: gate
metric: sentinel.silent_zero
definition_version: benchmark.silent-zero-v1
relationships: []
---

# Benchmark silent-zero sentinel

## Decision Objective

Prevent the benchmark from accepting ratio output from an instrument that read
none of a non-empty population without saying so.

## Population and Scope

Include all ratio metrics emitted by every completed producer in a benchmark
run. Exclude count metrics and genuinely empty populations.

## Measure Definition

Count metrics with non-zero examined, zero matched, and no diagnostic,
definition `benchmark.silent-zero-v1`.

## Collection and Provenance

Use complete raw payloads and retain verified capabilities, tool and engine
versions, declaration and corpus digests, configuration, and timestamp.

## Environment and Sampling

Mutation tests must demonstrate rejection when the diagnostic or capability is
removed.

## Interpretation and Limitations

This proves a narrow instrument-integrity property, not finding correctness.

## Comparison and Enforcement

Gate at exactly zero. Incomplete producers or missing capabilities are refusals,
not passing zeros.
