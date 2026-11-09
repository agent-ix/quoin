---
id: TASK-021
title: "Build fan-out projections (churn withdrawn, PLAT-1213)"
type: Task
status: done
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

# TASK-021: Build fan-out projections (churn withdrawn, PLAT-1213)

## Scope

Normalize the accepted export and retained binding store into immutable indexes, then derive exact
suite fan-out without losing unresolved records.

## Subtasks

- [x] Index live obligation owners and suite bindings with exact deduplication keys.
- [x] Emit one fan-out row per suite, including unresolved binding ids and gaps.
- [x] ~~Deduplicate affirmation copies~~ Withdrawn (PLAT-1213): the churn view and affirmation history are removed.
- [x] Property-test ordering, duplication, missing-obligation, and permutation cases.

## Deliverables

- Pure fan-out analyzer in `src/graph-analysis/`.
- TC-1249/TC-1250/TC-1254/TC-1255 tests.

## Notes

- `implements` relations are production scope, never fan-out evidence.
