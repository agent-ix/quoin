---
id: TASK-022
title: "Build deterministic reverse change-impact closure"
type: Task
status: not_started
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-021"
    type: depends_on
  - target: "ix://agent-ix/quoin/FR-062"
    type: references
  - target: "ix://agent-ix/quoin/TC-1251"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1252"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1253"
    type: verifies
---

# TASK-022: Build deterministic reverse change-impact closure

## Scope

Compute reverse dependency closure for requested requirement seeds using the effective relationship
selection, deterministic shortest paths, and exact joins to obligations, suites, and existing
auditor verdicts.

## Subtasks

- [ ] Validate default versus replacement relationship-kind selections against export vocabulary.
- [ ] Compute cycle-safe multi-source reverse closure with lexicographic shortest-path tie-breaking.
- [ ] Return no partial result for an unknown seed and `not_computed` for an unknown relation premise.
- [ ] Join live evidence and copy FR-032 verdicts into a field separate from exposure.

## Deliverables

- Pure change-impact analyzer and stable path model.
- TC-1251/TC-1252/TC-1253 tests.

## Notes

- Reachability never promotes, suppresses, or replaces an auditor verdict.
