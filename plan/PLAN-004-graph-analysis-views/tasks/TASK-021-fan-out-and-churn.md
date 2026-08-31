---
id: TASK-021
title: "Build fan-out and reaffirmation-churn projections"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-020"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-062"
    type: references
  - target: "ix://agent-ix/quoin/TC-1249"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1250"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1254"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1255"
    type: verifies
---

# TASK-021: Build fan-out and reaffirmation-churn projections

## Scope

Normalize the accepted export and retained binding store into immutable indexes, then derive exact
suite fan-out and obligation-level reaffirmation churn without losing unresolved records.

## Subtasks

- [ ] Index live obligation owners and suite bindings with exact deduplication keys.
- [ ] Emit one fan-out row per suite, including unresolved binding ids and gaps.
- [ ] Deduplicate affirmation copies while retaining affected suites and zero-event obligations.
- [ ] Property-test ordering, duplication, missing-obligation, and permutation cases.

## Deliverables

- Pure fan-out/churn analyzers in `src/graph-analysis/`.
- TC-1249/TC-1250/TC-1254/TC-1255 tests.

## Notes

- `implements` relations are production scope, never fan-out evidence.
