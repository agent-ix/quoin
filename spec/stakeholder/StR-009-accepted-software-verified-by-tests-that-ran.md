---
id: StR-009
title: "Acceptance rests on tests that ran and passed, not tests that are annotated"
type: StR
relationships:
  - target: "ix://agent-ix/quoin/FR-120"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/FR-121"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/NFR-028"
    type: "satisfied_by"
  - target: "ix://agent-ix/quoin/NFR-029"
    type: "satisfied_by"
---

# StR-009: Acceptance rests on tests that ran and passed, not tests that are annotated

## Stakeholder Need

Someone accepting software requires that Quoin shall bind a requirement to a test
only through a result that the test run itself produced and that names the
requirement, so that "verified" means a test with that trace id executed and
passed, not that a test carrying an annotation exists.

## Rationale

A test matrix built from source annotations is true the moment a test is written
and stays true while the test fails, is skipped, or is never run. Binding by
name-matching after the run is no better: it asserts a requirement the run never
named, and a rename or a parametrized case can attach a result to the wrong one.
A result that carries its own ids can be read, archived and re-read without the
source tree it came from.

## Validation Criteria

| ID | Criteria | Validation |
| --- | --- | --- |
| StR-009-VC-1 | A recorded run binds an obligation only through ids the run emitted. | Test |
| StR-009-VC-2 | A failing, erroring or skipped test discharges nothing. | Test |
| StR-009-VC-3 | The result file is interpretable without the source tree. | Test |

## Stakeholders

Assurance reviewers and release approvers who accept a requirement as verified.
Affected parties are maintainers of the pytest, vitest/jest and Rust emitters.

## Dependencies

FR-120 owns the mark contract, FR-121 the emitter mechanism constraints, NFR-028
inertness and cost, and NFR-029 self-contained evidence. ADR-100 records why the
ids are emitted rather than derived.
