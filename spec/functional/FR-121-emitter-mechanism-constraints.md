---
id: FR-121
title: "Emitter mechanism constraints"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-120"
    type: "extends"
  - target: "ix://agent-ix/quoin/US-023"
    type: "traces_to"
---

# FR-121: Emitter mechanism constraints

## Description

Whatever mechanism an emitter uses to carry a trace id from the source marker into
the result output, it SHALL hold the constraints below. They decide between the
candidate Rust mechanisms (a structured line in captured output, a custom harness
over a link-time registry, a runtime sidecar file) and bind the pytest and
vitest/jest emitters equally. The result file an emitter produces SHALL satisfy
[FR-120](./FR-120-trace-marks-in-test-result-output.md); a carrier other than the
`<property>` mark needs its own requirement amending FR-120 before an emitter
ships it.

## Rationale

The panicking test is the most interesting outcome and the one a naive emitter
loses. A concurrent harness corrupts a shared output stream or file. An emitter
that is active in an ordinary build changes what ships to people who never asked
for evidence.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-121-AC-1 | With emission not enabled, an ordinary test run produces the same compiled artifacts and the same result output as before the emitter was installed. | Test |
| FR-121-AC-2 | Emission is enabled by an explicit switch (a cargo feature or an environment variable, or the runner's equivalent), never by the mere presence of the marker. | Test |
| FR-121-AC-3 | A test that panics or throws records `fail` with its trace ids; it is not absent from the result. | Test |
| FR-121-AC-4 | Under the runner's default parallel execution, every tagged test's ids appear exactly once and unmixed with another test's. | Test |
| FR-121-AC-5 | The Rust emitter produces JUnit through `cargo-nextest` or an equivalent harness, because stock `cargo test` has no stable machine-readable output. | Test |
| FR-121-AC-6 | No mechanism writes a sidecar file that is merged into the results after the run. | Inspection |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-121-CON-1 | A mechanism SHALL NOT add a runtime dependency to a build that is not recording evidence. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-120](./FR-120-trace-marks-in-test-result-output.md), [NFR-028](../non-functional/NFR-028-inert-and-low-cost-emission.md).
- **Downstream**: the Rust emitter (PLAT-333).
