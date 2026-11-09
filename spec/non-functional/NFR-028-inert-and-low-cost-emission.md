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

A build that is not recording evidence SHALL be byte-for-byte what it is without
the emitter, and enabling emission SHALL NOT measurably slow the suite.

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
| Differing bytes in compiled output, emission off versus no emitter | 0 | 0 | Test |
| Suite wall-clock overhead with emission on | under 2% | under 5% | Benchmark |

## Verification

Build the same tree with and without the emitter, emission off, and compare the
produced artifacts. Run a fixed suite with emission on and off several times and
compare median wall-clock time.

## Dependencies

- **Upstream**: [ADR-001](../adrs/ADR-001-execution-evidence-is-emitted-by-the-run.md)
- **Downstream**: the emitter implementations
