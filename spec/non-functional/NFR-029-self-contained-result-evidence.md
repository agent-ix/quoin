---
id: NFR-029
title: "A result artifact is interpretable without its source tree"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quoin/FR-120"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
---

# NFR-029: A result artifact is interpretable without its source tree

## Statement

Every test result artifact an emitter produces SHALL carry, in the artifact
itself, each tagged test's symbol, file path, outcome and trace ids (except that a case recorded under FR-120 E-4 without a `trace` property carries no ids), so that the
artifact is interpretable without the source tree that produced it.

## Scope

- Applies to: JUnit and normalized JSON result files produced by trace emitters.
- Not applied to: results from a runner that has no emitter.

## Rationale

A result that is meaningful only beside the source at one commit forces an archive
to retain both and to prove they correspond. A self-contained result needs
neither.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|--------|--------|-----------|--------|
| Tagged cases, other than FR-120 E-4 id-less cases, whose symbol, path, outcome and ids are not all in the artifact | 0 | 0 | Test |
| Source files read to record a result | 0 | 0 | Test |

## Verification

Record an emitter's result file in an empty directory with no source tree present
and assert that every tagged case, other than an FR-120 E-4 id-less case, is recorded with its symbol, path, outcome and
ids.

## Dependencies

- **Upstream**: [FR-120](../functional/FR-120-trace-marks-in-test-result-output.md)
