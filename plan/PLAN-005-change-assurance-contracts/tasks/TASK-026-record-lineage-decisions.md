---
id: TASK-026
title: "Record lineage and decisions"
type: Task
status: done
track: A
priority: P0
relationships:
  - target: "ix://agent-ix/quoin/TASK-025"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-063"
    type: "references"
  - target: "ix://agent-ix/quoin/TC-1268"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1269"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1270"
    type: "verifies"
  - target: "ix://agent-ix/quoin/TC-1271"
    type: "verifies"
---

# TASK-026: Record lineage and decisions

## Scope

Seal and retain immutable record revisions, verify strict N-1 lineage, and
validate exact retained ix-flow human decisions without invoking ix-flow.

## Subtasks

- [x] Write generated genesis/successor/adversarial lineage tests.
- [x] Implement digest-addressed record persistence and parent preservation.
- [x] Implement pure decision-event matching and static side-effect boundaries.

## Deliverables

- Record store, lineage verifier, decision matcher, and TC-1268..TC-1271 tests.
