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

Where emission is enabled, an emitter SHALL record each tagged test case once,
with its trace ids and outcome, in the result shape of
[FR-120](./FR-120-trace-marks-in-test-result-output.md).

While emission is not enabled, an emitter SHALL leave the build and the test run
unchanged.

These constraints decide between the candidate Rust mechanisms (a structured line
in captured output, a custom harness over a link-time registry, a runtime sidecar
file) and bind the pytest and vitest/jest emitters equally. The criteria describe
emitters that live outside quoin, so each is allocated to the repository that
owns the emitter, and quoin's matrix carries them as cross-repo rows. A carrier
other than the `<property>` mark needs its own requirement amending FR-120 before
an emitter ships it.

## Rationale

The panicking test is the most interesting outcome and the one a naive emitter
loses. A concurrent harness corrupts a shared output stream or file. An emitter
that is active in an ordinary build changes what ships to people who never asked
for evidence.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-121-AC-1 | With the emitter's enable switch absent, the compiled artifacts of a tree are byte-identical to those of the same tree with the emitter removed, and no `trace` property appears in the result output. | Test (cross-repo: emitter repositories; PLAT-333 for Rust) |
| FR-121-AC-2 | Emission is enabled by an explicit switch (a cargo feature or an environment variable, or the runner's equivalent), never by the mere presence of the marker. | Test (cross-repo: emitter repositories; PLAT-333 for Rust) |
| FR-121-AC-3 | A test that panics or throws records `fail` with its trace ids; it is not absent from the result. | Test (cross-repo: emitter repositories; PLAT-333 for Rust) |
| FR-121-AC-4 | Under the runner's default parallel execution, every tagged test's ids appear exactly once and unmixed with another test's. | Test (cross-repo: emitter repositories; PLAT-333 for Rust) |
| FR-121-AC-5 | The Rust run writes JUnit in which each case carries `name` and `classname` equal to the extractor's `qualified_name`, a `file` attribute, and the `trace` property, produced by the run's own harness with no post-run rewrite of the file. The runner or harness that meets this shape is chosen in PLAT-333. | Inspection (cross-repo: PLAT-333) |
| FR-121-AC-6 | No mechanism writes a sidecar file that is merged into the results after the run. | Inspection (cross-repo: emitter repositories) |

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-121-CON-1 | A mechanism SHALL NOT add a runtime dependency to a build that is not recording evidence. | Design | Inspection |

## Dependencies

- **Upstream**: [FR-120](./FR-120-trace-marks-in-test-result-output.md), [NFR-028](../non-functional/NFR-028-inert-and-low-cost-emission.md).
- **Downstream**: the Rust emitter (PLAT-333).
