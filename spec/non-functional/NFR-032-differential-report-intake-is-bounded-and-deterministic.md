---
id: NFR-032
title: "Differential report intake is bounded and deterministic"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quoin/FR-150"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
---

# NFR-032: Differential report intake is bounded and deterministic

## Statement

The differential report adapter SHALL bound the work it does on any input by the input's size, refuse an input over
16 MiB before parsing it, and return the same result for the same bytes on every run.

## Scope

- Applies to: the `differential-report` adapter of FR-150, including its refusal paths.
- Not applied to: the producer, which bounds its own report through its solver limits.

## Rationale

A v2 report embeds the raw output of two solver processes as hex, so its size is set by a producer's limits and
not by quoin. An intake that parses without a bound lets one oversized or hostile file exhaust the recording
process, and an intake whose result varies with time, path or member order records different evidence for the same
report.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Bytes of an over-limit input parsed before refusal | 0 | 0 | Test |
| Differing results across repeated runs over identical bytes, across paths and environments | 0 | 0 | Test |
| Panics on any malformed, truncated or oversized input in the fixture set | 0 | 0 | Test |

## Verification

Covered by FR-150-AC-8, FR-150-AC-10 and FR-150-AC-11, executed as TC-001.
