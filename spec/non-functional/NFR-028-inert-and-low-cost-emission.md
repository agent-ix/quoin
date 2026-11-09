---
id: NFR-028
title: "Emission is inert by default and does not slow the suite"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quoin/FR-121"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
---

# NFR-028: Emission is inert by default and does not slow the suite

## Statement

A trace emitter whose enable switch is absent SHALL leave the compiled output byte-identical to a build without the emitter.

While emission is enabled, a trace emitter SHALL add under 5% to the median wall-clock time of a suite.

## Scope

- Applies to: every trace emitter (pytest, vitest/jest, Rust).
- Operational context: ordinary local and CI runs with emission off; evidence runs with emission on.

## Rationale

For Rust the marker is gone once the compiler finishes expanding it, and that
guarantee must hold for everyone who is not recording evidence. This requirement
decides mechanism choice, so it is stated before an emitter is written.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Differing bytes in compiled output, emission off versus no emitter | 0 | 0 | Test (cross-repo: emitter repositories) |
| Suite wall-clock overhead with emission on | under 2% | under 5% | Benchmark (cross-repo: emitter repositories) |

## Verification

Build the same tree with and without the emitter, emission off, and compare the
produced artifacts. Run a fixed suite with emission on and off several times and
compare median wall-clock time.

## Dependencies

- **Upstream**: [ADR-100](../adrs/ADR-100-execution-evidence-is-emitted-by-the-run.md)
- **Downstream**: the emitter implementations
