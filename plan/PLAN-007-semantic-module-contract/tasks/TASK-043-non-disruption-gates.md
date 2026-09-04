---
id: TASK-043
title: "Non-disruption gates"
type: Task
status: todo
track: C
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-041"
    type: depends_on
  - target: "ix://agent-ix/quoin/TASK-042"
    type: depends_on
  - target: "ix://agent-ix/quoin/NFR-017"
    type: references
  - target: "ix://agent-ix/quoin/TC-1379"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1380"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1381"
    type: verifies
  - target: "ix://agent-ix/quoin/TC-1382"
    type: verifies
---

# TASK-043: Non-disruption gates

## Scope

Run the NFR-017 gates over the finished slice.

## Subtasks

- [ ] Load every `default-modules.yaml` module with the new loader; assert no new diagnostic (TC-1379).
- [ ] Sweep the fixture corpus; assert only `warning`-severity semantic findings (TC-1380).
- [ ] Changed-path test excluding corpus repositories (TC-1381); schema `required` diff (TC-1382).

## Deliverables

- Gate tests.

## Notes

- No corpus repository is written.
